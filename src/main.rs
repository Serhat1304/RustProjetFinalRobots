// Importation des modules internes
mod carte;
mod robot;
mod systemes;
mod utils;

use bevy::prelude::*;
use carte::{generer_carte, PositionStation, Carte, SeedCarte, DepotStation};
use robot::{creer_robots, deplacer_robots};
use systemes::{initialiser_camera, configurer_minuterie_robot, initialiser_robots_crees, synchroniser_pixels_carte};
use utils::{obtenir_seed_depuis_arguments, generer_seed_aleatoire};

fn main() {
    // On récupère le seed depuis les arguments ou on en génère un aléatoirement
    let seed = obtenir_seed_depuis_arguments().unwrap_or_else(generer_seed_aleatoire);
    println!("Seed utilisée : {}", seed);

    // Configuration de l'application Bevy
    App::new()
        .add_plugins(DefaultPlugins)
        // Insertion des ressources de seed et du dépôt de la station (pour les découvertes et le stock de ressources)
        .insert_resource(SeedCarte { seed })
        .insert_resource(DepotStation {
            decouvertes: Vec::new(),
            stock_energie: 0,
            stock_minerai: 0,
        })
        // Systèmes de démarrage
        .add_systems(Startup, initialiser_camera)
        .add_systems(Startup, generer_carte)
        .add_systems(Startup, configurer_minuterie_robot)
        .add_systems(Startup, initialiser_robots_crees)
        // Systèmes d'update
        .add_systems(Update, creer_robots)
        .add_systems(Update, deplacer_robots)
        .add_systems(Update, synchroniser_pixels_carte.after(deplacer_robots))
        .run();
}
