mod carte;
mod robot;
mod systemes;
mod utils;

use bevy::prelude::*;
use carte::{generer_carte, PositionStation, Carte, SeedCarte, DepotStation};
use robot::{creer_robots, deplacer_robots};
use systemes::{
    initialiser_camera, configurer_minuterie_robot, initialiser_robots_crees, synchroniser_pixels_carte,
    traiter_evenements,
};
use utils::{obtenir_seed_depuis_arguments, generer_seed_aleatoire, Evenements};

fn main() {
    // Récupération du seed depuis les arguments ou génération aléatoire
    let seed = obtenir_seed_depuis_arguments().unwrap_or_else(generer_seed_aleatoire);
    println!("Seed utilisée : {}", seed);

    App::new()
        .add_plugins(DefaultPlugins)
        // Insertion des ressources : seed, dépôt et liste d'événements
        .insert_resource(SeedCarte { seed })
        .insert_resource(DepotStation {
            decouvertes: Vec::new(),
            stock_energie: 0,
            stock_minerai: 0,
        })
        .insert_resource(Evenements::default())
        // Systèmes de démarrage
        .add_systems(Startup, initialiser_camera)
        .add_systems(Startup, generer_carte)
        .add_systems(Startup, configurer_minuterie_robot)
        .add_systems(Startup, initialiser_robots_crees)
        // Systèmes d'update
        .add_systems(Update, creer_robots)
        .add_systems(Update, deplacer_robots)
        .add_systems(Update, synchroniser_pixels_carte.after(deplacer_robots))
        .add_systems(Update, traiter_evenements)
        .run();
}
