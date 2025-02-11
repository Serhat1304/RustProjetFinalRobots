mod map;
mod obstacles;
mod station;
mod utils;
mod common;

use bevy::prelude::*;
use utils::{generer_seed_aleatoire, obtenir_seed_depuis_arguments};
use map::generer_map;
use station::initialiser_station;

#[derive(Resource)]
struct SeedCarte {
    seed: u64,
}

fn initialiser_map(mut commandes: Commands) {
    commandes.spawn(Camera2dBundle {
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 1000.0)),
        ..Default::default()
    });
}

fn main() {
    let seed = obtenir_seed_depuis_arguments().unwrap_or_else(generer_seed_aleatoire);
    println!("Seed utilisée : {}", seed);

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Exploration des Robots".to_string(),
                resolution: (1000.0, 800.0).into(),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .insert_resource(SeedCarte { seed })
        .add_systems(Startup, (initialiser_map, initialiser_station, generer_map))
        .run();
}