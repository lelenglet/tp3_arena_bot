// ─── Partie 3 : Stratégie de déplacement ─────────────────────────────────────
//
// Objectif : définir un trait Strategy et l'utiliser via Box<dyn Strategy>
// pour choisir le prochain mouvement du bot à chaque tick.
//
// Concepts exercés : dyn Trait, Box<dyn Strategy>, Send, dispatch dynamique.
//
// ─────────────────────────────────────────────────────────────────────────────

// TODO: Importer les types nécessaires de state.rs
// use crate::state::GameState;
use crate::state::GameState;
// TODO: Définir le trait Strategy.
//
// Le trait doit :
//   - Être object-safe (pas de generics dans les méthodes)
//   - Être Send (pour pouvoir être utilisé dans un contexte multi-thread)
//   - Avoir une méthode next_move qui retourne un déplacement optionnel
//
// pub trait Strategy: Send {
//     /// Décide du prochain mouvement en fonction de l'état du jeu.
//     ///
//     /// Retourne Some((dx, dy)) avec dx, dy ∈ {-1, 0, 1}, ou None pour rester sur place.
//     fn next_move(&self, state: &GameState) -> Option<(i8, i8)>;
// }
pub trait Strategy: Send {
    fn next_move(&self, state: &GameState) -> Option<(i8, i8)>;
}
// TODO: Implémenter NearestResourceStrategy.
//
// Cette stratégie se dirige vers la ressource la plus proche (distance de Manhattan).
//
// pub struct NearestResourceStrategy;
//
// impl Strategy for NearestResourceStrategy {
//     fn next_move(&self, state: &GameState) -> Option<(i8, i8)> {
//         // 1. Trouver la ressource la plus proche en distance de Manhattan :
//         //    distance = |resource.x - position.x| + |resource.y - position.y|
//         //
//         //    Indice : utilisez .iter().min_by_key(|r| ...)
//         //
//         // 2. Calculer la direction (dx, dy) vers cette ressource :
//         //    - Si resource.x > position.x → dx = 1
//         //    - Si resource.x < position.x → dx = -1
//         //    - Sinon dx = 0
//         //    - Idem pour dy
//         //
//         //    Indice : utilisez i16 pour les calculs puis .signum() puis cast en i8
//         //
//         // 3. Retourner Some((dx, dy)), ou None si aucune ressource
//         todo!()
//     }
// }

pub struct NearestResourceStrategy;

impl Strategy for NearestResourceStrategy {
    fn next_move(&self, state: &GameState) -> Option<(i8, i8)> {
        let nearest = state.resources.iter().min_by_key(|r| {
            let dx = (r.x as i16 - state.position.0 as i16).abs();
            let dy = (r.y as i16 - state.position.1 as i16).abs();
            dx + dy
        });

        match nearest {
            Some(ri) => {
                let dx = (ri.x as i16 - state.position.0 as i16).signum() as i8;
                let dy = (ri.y as i16 - state.position.1 as i16).signum() as i8;
                //println!("{}, {} - {}, {}", ri.x, ri.y, dx, dy);
                return Some((dx, dy));
            }
            None => None,
        }
    }
}

pub struct BFSStrategy;
use std::collections::{HashSet, VecDeque};

impl Strategy for BFSStrategy {
    fn next_move(&self, state: &GameState) -> Option<(i8, i8)> {
        let start = state.position;
        let obstacles: HashSet<(u16, u16)> = state.obstacles.iter().cloned().collect();
        let resource_coords: HashSet<(u16, u16)> =
            state.resources.iter().map(|r| (r.x, r.y)).collect();

        // queue stores: (current_x, current_y, first_step_x, first_step_y)
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        visited.insert(start);

        // Initialize queue with the 4 possible first moves
        for &(dx, dy) in &[(0, 1), (0, -1), (1, 0), (-1, 0)] {
            let nx = start.0 as i32 + dx as i32;
            let ny = start.1 as i32 + dy as i32;

            if nx >= 0 && nx < state.map_size.0 as i32 && ny >= 0 && ny < state.map_size.1 as i32 {
                let pos = (nx as u16, ny as u16);
                if !obstacles.contains(&pos) {
                    queue.push_back((pos, (dx as i8, dy as i8)));
                    visited.insert(pos);
                }
            }
        }

        while let Some(((curr_x, curr_y), first_step)) = queue.pop_front() {
            // Check if we reached a resource
            if resource_coords.contains(&(curr_x, curr_y)) {
                return Some(first_step);
            }

            // Explore neighbors
            for &(dx, dy) in &[(0, 1), (0, -1), (1, 0), (-1, 0)] {
                let nx = curr_x as i32 + dx as i32;
                let ny = curr_y as i32 + dy as i32;

                if nx >= 0
                    && nx < state.map_size.0 as i32
                    && ny >= 0
                    && ny < state.map_size.1 as i32
                {
                    let next_pos = (nx as u16, ny as u16);
                    if !visited.contains(&next_pos) && !obstacles.contains(&next_pos) {
                        visited.insert(next_pos);
                        queue.push_back((next_pos, first_step));
                    }
                }
            }
        }

        None // No reachable resources
    }
}

// ─── BONUS : Implémenter d'autres stratégies ────────────────────────────────
//
// Exemples :
//   - RandomStrategy : mouvement aléatoire
//   - FleeStrategy : s'éloigne des autres agents
//   - HybridStrategy : combine plusieurs stratégies
//
// Utilisation dans main.rs :
//   let strategy: Box<dyn Strategy> = Box::new(NearestResourceStrategy);
//
// On peut changer de stratégie sans modifier le reste du code grâce au
// dispatch dynamique (Box<dyn Strategy>).
