// ─── Partie 2 : Pool de mineurs ──────────────────────────────────────────────
//
// Objectif : créer un pool de N threads qui cherchent des nonces en parallèle.
//
// Concepts exercés : thread::spawn, mpsc::channel, Arc, move closures.
//
// Architecture :
//
//   Thread principal                        Threads mineurs (x N)
//        |                                        |
//        |── mpsc::Sender<MineRequest> ──────────>|  (challenges à résoudre)
//        |                                        |
//        |<── mpsc::Sender<MineResult> ──────────>|  (solutions trouvées)
//        |                                        |
//
// Chaque thread mineur :
//   1. Attend un MineRequest sur son channel
//   2. Appelle pow::pow_search() avec un start_nonce différent
//   3. Si un nonce est trouvé, envoie un MineResult
//
// ─────────────────────────────────────────────────────────────────────────────

use std::sync::Arc;

use uuid::Uuid;

use crate::pow::pow_valid;

/// Requête de minage envoyée aux threads mineurs.
#[derive(Debug, Clone)]
pub struct MineRequest {
    pub seed: String,
    pub tick: u64,
    pub resource_id: Uuid,
    pub agent_id: Uuid,
    pub target_bits: u8,
}

/// Résultat renvoyé par un mineur quand il trouve un nonce valide.
#[derive(Debug, Clone)]
pub struct MineResult {
    pub tick: u64,
    pub resource_id: Uuid,
    pub nonce: u64,
}

// TODO: Définir la structure MinerPool.
//
// Elle doit contenir :
//   - Le Sender pour envoyer des MineRequest aux threads
//   - Le Receiver pour récupérer les MineResult
//
// Indice : les types sont :
//   std::sync::mpsc::Sender<MineRequest>
//   std::sync::mpsc::Receiver<MineResult>
//
// pub struct MinerPool {
//     ...
// }

pub struct MinerPool {
    sender: std::sync::mpsc::Sender<MineRequest>,
    receiver: std::sync::mpsc::Receiver<MineResult>,
}

// TODO: Implémenter MinerPool.
//
// impl MinerPool {
//     /// Crée un pool de `n` threads mineurs.
//     ///
//     /// Chaque thread :
//     ///   1. Possède un Receiver<MineRequest> (partagé via Arc<Mutex<>>)
//     ///   2. Possède un Sender<MineResult> (cloné pour chaque thread)
//     ///   3. Boucle : recv() → pow_search() → send() si trouvé
//     ///
//     /// Indices :
//     ///   - Un seul Receiver existe par channel. Pour le partager entre N threads,
//     ///     il faut le wrapper dans Arc<Mutex<Receiver<MineRequest>>>.
//     ///   - Chaque thread clone le Arc pour accéder au Receiver.
//     ///   - pow::pow_search() prend un start_nonce et un batch_size.
//     ///     Utilisez rand::random::<u64>() comme start_nonce pour que chaque
//     ///     appel explore une zone différente.
//     ///   - Batch size recommandé : 100_000
//     ///
//     pub fn new(n: usize) -> Self {
//         // Créer les 2 channels :
//         //   - (request_tx, request_rx) pour envoyer les challenges
//         //   - (result_tx, result_rx) pour recevoir les solutions
//         //
//         // Wrapper request_rx dans Arc<Mutex<>>
//         //
//         // Pour chaque thread (0..n) :
//         //   - Cloner le Arc<Mutex<Receiver<MineRequest>>>
//         //   - Cloner le result_tx
//         //   - thread::spawn(move || { ... boucle de minage ... })
//         //
//         // Retourner MinerPool { request_tx, result_rx }
//         todo!()
//     }
//
//     /// Envoie un challenge de minage au pool.
//     pub fn submit(&self, request: MineRequest) {
//         todo!()
//     }
//
//     /// Tente de récupérer un résultat sans bloquer.
//     pub fn try_recv(&self) -> Option<MineResult> {
//         todo!()
//     }
// }

impl MinerPool {
    pub fn new(n: usize) -> Self {
        let (request_tx, request_rx) = std::sync::mpsc::channel::<MineRequest>();
        let (result_tx, result_rx) = std::sync::mpsc::channel::<MineResult>();
        let wrapped_request_receiver = std::sync::Arc::new(std::sync::Mutex::new(request_rx));
        for i in 0..n {
            let cloned_amrm = Arc::clone(&wrapped_request_receiver);
            let cloned_tx = result_tx.clone();
            let thread_id = i as u64;
            let total_threads = n as u64;

            std::thread::spawn(move || {
                let mut current_req: Option<MineRequest> = None;
                let mut nonce = thread_id; // Each thread starts at a unique offset

                loop {
                    // 1. Check for a NEW job without blocking the mining
                    if let Ok(lock) = cloned_amrm.try_lock() {
                        if let Ok(new_req) = lock.try_recv() {
                            current_req = Some(new_req);
                            nonce = thread_id; // Reset nonce for the new resource
                        }
                    }

                    if let Some(ref req) = current_req {
                        // 2. Mine a batch
                        // We don't use pow_search here because we want to control the increment
                        for _ in 0..100_000 {
                            if pow_valid(
                                &req.seed,
                                req.tick,
                                req.resource_id,
                                req.agent_id,
                                nonce,
                                req.target_bits,
                            ) {
                                let _ = cloned_tx.send(MineResult {
                                    tick: req.tick,
                                    resource_id: req.resource_id,
                                    nonce,
                                });
                                current_req = None; // Stop mining this one
                                break;
                            }
                            // The magic of striping:
                            // Thread 0: 0, 4, 8, 12...
                            // Thread 1: 1, 5, 9, 13...
                            nonce += total_threads;
                        }
                    } else {
                        // No job? Wait for one (blocking)
                        if let Ok(lock) = cloned_amrm.lock() {
                            if let Ok(new_req) = lock.recv() {
                                current_req = Some(new_req);
                                nonce = thread_id;
                            }
                        }
                    }
                }
            });
        }
        MinerPool {
            sender: request_tx,
            receiver: result_rx,
        }
    }

    pub fn submit(&self, request: MineRequest) {
        let _ = self.sender.send(request);
    }

    pub fn try_recv(&self) -> Option<MineResult> {
        match self.receiver.try_recv() {
            Ok(res) => Some(res),
            Err(_) => None,
        }
    }
}
