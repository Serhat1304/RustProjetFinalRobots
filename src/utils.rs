use bevy::prelude::*;
use noise::Perlin;
use rand::{prelude::*, SeedableRng};
use std::collections::{HashSet, VecDeque, HashMap};
use crate::carte::{Carte, TypePixel, LARGEUR_CARTE, HAUTEUR_CARTE};
use crate::carte::Decouverte;
use crate::carte::DepotStation;

/// Calcule un chemin entre deux points sur la carte en utilisant l'algorithme BFS.
/// Retourne Some(chemin) si un chemin est trouvé, ou None sinon.
pub fn calculer_chemin_bfs(
    carte: &Carte,
    depart: (isize, isize),
    arrivee: (isize, isize),
) -> Option<Vec<(isize, isize)>> {
    if depart == arrivee {
        return Some(vec![depart]);
    }
    if carte.est_obstacle(depart.0, depart.1) || carte.est_obstacle(arrivee.0, arrivee.1) {
        return None;
    }

    let mut visites = HashSet::new();
    let mut file = VecDeque::new();
    let mut came_from = HashMap::new();

    visites.insert(depart);
    file.push_back(depart);

    let directions = [(0, 1), (0, -1), (1, 0), (-1, 0)];

    while let Some(courant) = file.pop_front() {
        for (dx, dy) in directions.iter() {
            let nx = courant.0 + dx;
            let ny = courant.1 + dy;
            if !carte.est_obstacle(nx, ny) && !visites.contains(&(nx, ny)) {
                visites.insert((nx, ny));
                came_from.insert((nx, ny), courant);
                file.push_back((nx, ny));

                if (nx, ny) == arrivee {
                    let mut chemin = vec![(nx, ny)];
                    let mut courant_retour = (nx, ny);
                    while courant_retour != depart {
                        courant_retour = came_from[&courant_retour];
                        chemin.push(courant_retour);
                    }
                    chemin.reverse();
                    return Some(chemin);
                }
            }
        }
    }
    None
}

/// Enregistre une découverte dans le dépôt de la station.
/// Si une découverte existe déjà à la même position, un message est affiché.
pub fn enregistrer_decouverte(depot: &mut DepotStation, decouverte: Decouverte) {
    if let Some(existante) = depot.decouvertes.iter().find(|d| d.x == decouverte.x && d.y == decouverte.y) {
        if existante.resource != decouverte.resource {
            println!(
                "Conflit détecté pour la ressource en ({}, {}): {:?} vs {:?}",
                decouverte.x, decouverte.y, existante.resource, decouverte.resource
            );
        } else {
            println!(
                "Découverte déjà enregistrée pour la ressource en ({}, {})",
                decouverte.x, decouverte.y
            );
        }
    } else {
        depot.decouvertes.push(decouverte.clone());
        println!("Découverte enregistrée : {:?}", decouverte);
    }
}

/// Récupère le seed depuis les arguments de la ligne de commande, si présent.
pub fn obtenir_seed_depuis_arguments() -> Option<u64> {
    let arguments: Vec<String> = std::env::args().collect();
    if arguments.len() > 1 {
        arguments[1].parse::<u64>().ok()
    } else {
        None
    }
}

/// Génère un seed aléatoire.
pub fn generer_seed_aleatoire() -> u64 {
    rand::thread_rng().gen::<u64>()
}

/// Énumération des événements pour l'event sourcing.
#[derive(Debug)]
pub enum Evenement {
    RobotDeplace { robot_id: u32, from: (isize, isize), to: (isize, isize) },
    RessourceCollectee { robot_id: u32, resource: TypePixel, position: (isize, isize) },
    NouveauRobotCree { robot_role: crate::robot::RoleRobot, modules: Vec<crate::robot::ModuleRobot> },
}

/// Ressource contenant la liste des événements enregistrés.
#[derive(Resource, Default)]
pub struct Evenements {
    pub events: Vec<Evenement>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::carte::Carte;
    use crate::carte::TypePixel;

    #[test]
    fn test_calculer_chemin_bfs_simple() {
        // Création d'une carte 3x3 sans obstacles
        let donnees = vec![
            vec![TypePixel::Vide; 3],
            vec![TypePixel::Vide; 3],
            vec![TypePixel::Vide; 3],
        ];
        let carte = Carte { donnees, largeur: 3, hauteur: 3 };
        let chemin = calculer_chemin_bfs(&carte, (0, 0), (2, 2));
        assert!(chemin.is_some());
        let chemin = chemin.unwrap();
        assert_eq!(chemin.first(), Some(&(0, 0)));
        assert_eq!(chemin.last(), Some(&(2, 2)));
    }

    #[test]
    fn test_calculer_chemin_bfs_obstacle() {
        // Création d'une carte 3x3 avec obstacles bloquant le chemin direct
        let donnees = vec![
            vec![TypePixel::Vide, TypePixel::Obstacle, TypePixel::Vide],
            vec![TypePixel::Vide, TypePixel::Obstacle, TypePixel::Vide],
            vec![TypePixel::Vide, TypePixel::Vide, TypePixel::Vide],
        ];
        let carte = Carte { donnees, largeur: 3, hauteur: 3 };
        let chemin = calculer_chemin_bfs(&carte, (0, 0), (2, 2));
        assert!(chemin.is_some());
        let chemin = chemin.unwrap();
        // Vérifier que le chemin ne traverse pas une case avec obstacle
        for &(x, y) in &chemin {
            if x == 1 {
                assert!(carte.donnees[y as usize][x as usize] != TypePixel::Obstacle);
            }
        }
    }

    #[test]
    fn test_enregistrer_decouverte() {
        use crate::carte::{DepotStation, Decouverte};
        let mut depot = DepotStation { decouvertes: Vec::new(), stock_energie: 0, stock_minerai: 0 };
        let decouverte = Decouverte { resource: TypePixel::Energie, x: 1, y: 1 };
        enregistrer_decouverte(&mut depot, decouverte.clone());
        assert_eq!(depot.decouvertes.len(), 1);
        // Ajouter une découverte identique ne doit pas augmenter le nombre
        enregistrer_decouverte(&mut depot, decouverte);
        assert_eq!(depot.decouvertes.len(), 1);
    }

    #[test]
    fn test_evenements_default() {
        let evenements = Evenements::default();
        assert_eq!(evenements.events.len(), 0);
    }

    #[test]
    fn test_evenement_robot_deplace() {
        let evt = Evenement::RobotDeplace { robot_id: 42, from: (0, 0), to: (1, 1) };
        match evt {
            Evenement::RobotDeplace { robot_id, from, to } => {
                assert_eq!(robot_id, 42);
                assert_eq!(from, (0, 0));
                assert_eq!(to, (1, 1));
            },
            _ => panic!("Mauvais type d'événement"),
        }
    }
}
