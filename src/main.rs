// Le squelette contient du code fourni pas encore utilisé — c'est normal.
#![allow(dead_code)]

mod miner;
mod pow;
mod protocol;
mod state;
mod strategy;

// Ces imports seront utilisés dans votre implémentation.
#[allow(unused_imports)]
use std::sync::{Arc, Mutex};
#[allow(unused_imports)]
use std::thread;
#[allow(unused_imports)]
use std::time::Duration;

use tungstenite::{connect, Message};
#[allow(unused_imports)]
use uuid::Uuid;

use protocol::{ClientMsg, ServerMsg};

use crate::miner::MinerPool;
use crate::state::GameState;
use crate::strategy::{BFSStrategy, Strategy};

// ─── Configuration ──────────────────────────────────────────────────────────

const SERVER_URL: &str = "ws://localhost:4004/ws";
const TEAM_NAME: &str = "les_chèvres";
const AGENT_NAME: &str = "yodelayheehoo";
const NUM_MINERS: usize = 4;

fn main() {
    println!("[*] Connexion à {SERVER_URL}...");
    let (mut ws, _response) = connect(SERVER_URL).expect("impossible de se connecter au serveur");
    println!("[*] Connecté !");

    // ── Attendre le Hello ────────────────────────────────────────────────
    #[allow(unused_variables)] // Vous utiliserez agent_id dans votre implémentation.
    let agent_id: Uuid = match read_server_msg(&mut ws) {
        Some(ServerMsg::Hello { agent_id, tick_ms }) => {
            println!("[*] Hello reçu : agent_id={agent_id}, tick={tick_ms}ms");
            agent_id
        }
        other => panic!("premier message inattendu : {other:?}"),
    };

    // ── S'enregistrer ────────────────────────────────────────────────────
    send_client_msg(
        &mut ws,
        &ClientMsg::Register {
            team: TEAM_NAME.into(),
            name: AGENT_NAME.into(),
        },
    );
    println!("[*] Enregistré en tant que {AGENT_NAME} (équipe {TEAM_NAME})");

    // ─────────────────────────────────────────────────────────────────────
    //  À PARTIR D'ICI, C'EST À VOUS DE JOUER !
    //
    //  Objectif : implémenter la boucle principale du bot.
    //
    //  Étapes suggérées :
    //
    //  1. Créer l'état partagé (state::SharedState)
    //     let shared_state = state::SharedState::new(agent_id);
    //
    //  2. Créer le pool de mineurs (miner::MinerPool)
    //     let miner_pool = miner::MinerPool::new(NUM_MINERS);
    //
    //  3. Créer la stratégie de déplacement
    //     let strategy: Box<dyn strategy::Strategy> = Box::new(strategy::NearestResourceStrategy);
    //
    //  4. Séparer la WebSocket en lecture/écriture
    //     Utiliser ws.into_inner() pour récupérer le TcpStream puis séparer
    //     via std::io::Read/Write. Sinon, approche plus simple ci-dessous :
    //
    //  ─── Approche simplifiée (recommandée) ─────────────────────────────
    //
    //  Utiliser ws.read() dans un thread dédié qui :
    //    a) parse les ServerMsg
    //    b) met à jour le SharedState
    //    c) envoie les PowChallenge au MinerPool via un channel
    //
    //  Le thread principal :
    //    a) vérifie si le MinerPool a trouvé une solution → envoie PowSubmit
    //    b) consulte la stratégie pour décider du prochain mouvement → envoie Move
    //    c) dort un court instant (ex: 50ms) pour ne pas surcharger
    //
    //  Contrainte : la WebSocket (tungstenite) n'est pas Send si on utilise
    //  la version par défaut. Vous devrez garder toutes les écritures WS
    //  dans le thread principal, et utiliser des channels pour communiquer
    //  depuis le thread lecteur.
    // ─────────────────────────────────────────────────────────────────────

    // TODO: Partie 1 — Créer le SharedState (voir state.rs)
    let shared_state: Arc<Mutex<GameState>> = state::new_shared_state(agent_id);

    // TODO: Partie 2 — Créer le MinerPool (voir miner.rs)
    let minerpool = MinerPool::new(NUM_MINERS);

    // TODO: Partie 3 — Créer la stratégie (voir strategy.rs)
    let strategy: Box<dyn Strategy> = Box::new(BFSStrategy);

    // TODO: Partie 4 — Lancer le thread lecteur WS
    //
    // Indice : il faut un channel pour recevoir les messages du thread lecteur
    // car la WebSocket ne peut pas être partagée entre threads.
    //
    // let (tx, rx) = std::sync::mpsc::channel::<ServerMsg>();
    //
    // Le thread lecteur lit les messages, met à jour le state, et forward
    // les messages importants via le channel.

    // Canal pour : Serveur -> Lecteur -> Principal (Messages entrants)
    let (tx_in, rx_in) = std::sync::mpsc::channel::<ServerMsg>();

    // Canal pour : Principal -> Lecteur -> Serveur (Messages sortants)
    let (tx_out, rx_out) = std::sync::mpsc::channel::<ClientMsg>();

    let cloned_state = std::sync::Arc::clone(&shared_state);
    let cloned_tx = tx_in.clone();
    thread::spawn(move || loop {
        while let Ok(message) = rx_out.try_recv() {
            send_client_msg(&mut ws, &message);
        }

        if let Some(message) = read_server_msg(&mut ws) {
            cloned_state.lock().unwrap().update(&message);
            let _ = cloned_tx.send(message);
        };
    });
    // TODO: Partie 5 — Boucle principale
    //
    // loop {
    //     // 1. Lire les messages du thread lecteur (rx.try_recv())
    //     //    - PowChallenge → envoyer au MinerPool
    //     //    - Win → afficher et quitter
    //     //    - Autres → déjà traités par le thread lecteur
    //
    //     // 2. Vérifier si le MinerPool a trouvé un nonce
    //     //    → envoyer ClientMsg::PowSubmit
    //
    //     // 3. Consulter la stratégie pour le prochain mouvement
    //     //    → envoyer ClientMsg::Move
    //
    //     // 4. Dormir un peu
    //     thread::sleep(Duration::from_millis(50));
    // }
    let mut last_tick_message = 0;
    loop {
        match rx_in.try_recv() {
            Ok(incoming_response) => match incoming_response {
                ServerMsg::PowChallenge {
                    tick,
                    seed,
                    resource_id,
                    x: _,
                    y: _,
                    target_bits,
                    expires_at: _,
                    value: _,
                } => {
                    if tick > last_tick_message {
                        minerpool.submit(miner::MineRequest {
                            seed,
                            tick,
                            resource_id,
                            agent_id,
                            target_bits,
                        });
                        last_tick_message = tick;
                    }
                }
                ServerMsg::Win { team } => {
                    println!("{} won", team);
                    break;
                }
                _ => (),
            },
            Err(_) => (),
        }

        //println!("[*] Communication avec mineur");
        match minerpool.try_recv() {
            Some(mr) => {
                let _ = tx_out.send(ClientMsg::PowSubmit {
                    tick: mr.tick,
                    resource_id: mr.resource_id,
                    nonce: mr.nonce,
                });
            }
            None => (),
        }

        //println!("[*] Communication avec strategy");
        let move_position = {
            let state = shared_state.lock().unwrap();
            strategy.next_move(&state)
        };
        match move_position {
            Some((dx, dy)) => {
                //println!("{}, {}", dx, dy);
                let _ = tx_out.send(ClientMsg::Move { dx: dx, dy: dy });
            }
            None => (),
        }

        thread::sleep(Duration::from_millis(250));
    }
}

// ─── Fonctions utilitaires (fournies) ───────────────────────────────────────

type WsStream = tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>;

/// Lit un message du serveur et le désérialise.
fn read_server_msg(ws: &mut WsStream) -> Option<ServerMsg> {
    match ws.read() {
        Ok(Message::Text(text)) => serde_json::from_str(&text).ok(),
        Ok(_) => None,
        Err(e) => {
            eprintln!("[!] Erreur WS lecture : {e}");
            None
        }
    }
}

/// Sérialise et envoie un message au serveur.
fn send_client_msg(ws: &mut WsStream, msg: &ClientMsg) {
    let json = serde_json::to_string(msg).expect("sérialisation échouée");
    ws.send(Message::Text(json.into()))
        .expect("envoi WS échoué");
}
