use bevy::prelude::*;
use noise::Perlin;
use rand::{prelude::*, SeedableRng};
use std::collections::{HashSet, VecDeque, HashMap};
use crate::carte::{Carte, TypePixel, LARGEUR_CARTE, HAUTEUR_CARTE};
use crate::carte::Decouverte;
use crate::carte::DepotStation;

/// Calcule un chemin entre deux points sur la carte en utilisant l'algorithme BFS.
/// Retourne Some(chemin) si un chemin existe ou None sinon.
pub fn calculer_chemin_bfs(carte: &Carte, depart: (isize, isize), arrivee: (isize, isize)) -> Option<Vec<(isize, isize)>> {
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
/// Si une découverte existe déjà à la même position, on affiche un message de conflit ou d'existence.
pub fn enregistrer_decouverte(depot: &mut DepotStation, decouverte: Decouverte) {
    if let Some(existante) = depot.decouvertes.iter().find(|d| d.x == decouverte.x && d.y == decouverte.y) {
        if existante.resource != decouverte.resource {
            println!("Conflit détecté pour la ressource en ({}, {}): {:?} vs {:?}", decouverte.x, decouverte.y, existante.resource, decouverte.resource);
        } else {
            println!("Découverte déjà enregistrée pour la ressource en ({}, {})", decouverte.x, decouverte.y);
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
