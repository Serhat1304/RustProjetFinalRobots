use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use rand::{prelude::*, SeedableRng};
use std::env;
// cargo run = génération aléatoire de la map
// cargo run -- xxxxxx = Génération d'un seed x donné

// Paramètres de la carte
const LARGEUR_CARTE: usize = 50;
const HAUTEUR_CARTE: usize = 30;
const TAILLE_CASE: f32 = 20.0;

// Seuil de bruit définissant les obstacles (plus haut = plus d'obstacles)
const SEUIL_OBSTACLE: f64 = 0.5;

// Taille maximale des obstacles en pixels connectés
// Pour éviter d'avoir des obstacles trop grands.
const MAX_TAILLE_OBSTACLE: usize = 5;

/// Enumération des types de pixel présents sur la carte
#[derive(Debug, Clone, Copy, PartialEq)]
enum TypePixel {
    Vide,
    Obstacle,
    Energie,
    Minerai,
    SiteScientifique,
    Station,
}

/// Composant Bevy pour les entités représentant un pixel de la carte
#[derive(Component)]
struct Tuile {
    type_tuile: TypePixel,
}

fn main() {
    // Vérifie si l'utilisateur a fourni une seed en argument ou en génère une aléatoire
    let seed = obtenir_seed_depuis_arguments().unwrap_or_else(generer_seed_aleatoire);
    println!("Seed utilisée : {}", seed);

    // Initialisation de Bevy avec la seed stockée
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(SeedCarte { seed }) // Stocke la seed pour garantir une génération reproductible
        .add_systems(Startup, initialiser_map)
        .add_systems(Startup, generer_map)
        .run();
}

/// Ressource stockant la seed
#[derive(Resource)]
struct SeedCarte {
    seed: u64,
}

/// si une seed a été fournie en argument, sinon retourne None
fn obtenir_seed_depuis_arguments() -> Option<u64> {
    let arguments: Vec<String> = env::args().collect();
    if arguments.len() > 1 {
        arguments[1].parse::<u64>().ok()
    } else {
        None
    }
}

/// Génère une seed aléatoire si aucune n'est fournie
fn generer_seed_aleatoire() -> u64 {
    rand::thread_rng().gen::<u64>()
}

/// Initialise la caméra dans la simulation
fn initialiser_map(mut commandes: Commands) {
    commandes.spawn(Camera2dBundle::default());
}

/// génère la carte avec les obstacles et les ressources
fn generer_map(mut commandes: Commands, seed_carte: Res<SeedCarte>) {
    println!("Seed Actuel: {}", seed_carte.seed);

    let bruit_perlin = Perlin::new(seed_carte.seed as u32);
    let mut generateur_aleatoire = StdRng::seed_from_u64(seed_carte.seed);

    let mut carte = vec![vec![TypePixel::Vide; LARGEUR_CARTE]; HAUTEUR_CARTE];

    // Génération des obstacles en utilisant le bruit de Perlin
    for y in 0..HAUTEUR_CARTE {
        for x in 0..LARGEUR_CARTE {
            let valeur_bruit = bruit_perlin.get([x as f64 * 0.1, y as f64 * 0.1]);

            if valeur_bruit > SEUIL_OBSTACLE {
                carte[y][x] = TypePixel::Obstacle;
            }
        }
    }

    // Limite la taille des obstacles pour éviter des zones trop grandes
    limiter_taille_obstacles(&mut carte);

    // Ajout aléatoire des ressources sur les tuiles vides
    for y in 0..HAUTEUR_CARTE {
        for x in 0..LARGEUR_CARTE {
            if carte[y][x] == TypePixel::Vide {
                carte[y][x] = match generateur_aleatoire.gen_range(0..100) {
                    0..=5 => TypePixel::Energie,        // 6% de chance
                    6..=10 => TypePixel::Minerai,      // 5% de chance
                    11..=14 => TypePixel::SiteScientifique, // 4% de chance
                    _ => TypePixel::Vide,
                };
            }
        }
    }

    // Placement de la station sur une case vide
    let (pos_x, pos_y) = placer_station(&mut carte, &mut generateur_aleatoire);
    println!("Station placée en ({}, {})", pos_x, pos_y);

    // 🔹 Création des entités Bevy pour afficher la carte
    for y in 0..HAUTEUR_CARTE {
        for x in 0..LARGEUR_CARTE {
            let couleur = match carte[y][x] {
                TypePixel::Obstacle => Color::rgb(0.2, 0.2, 0.2),
                TypePixel::Energie => Color::rgb(1.0, 1.0, 0.0),
                TypePixel::Minerai => Color::rgb(0.5, 0.3, 0.1),
                TypePixel::SiteScientifique => Color::rgb(0.0, 0.8, 0.8),
                TypePixel::Station => Color::rgb(1.0, 0.0, 0.0), // 🔴 Station en rouge
                TypePixel::Vide => Color::rgb(0.8, 0.8, 0.8),
            };

            commandes.spawn(SpriteBundle {
                sprite: Sprite {
                    color: couleur,
                    custom_size: Some(Vec2::splat(TAILLE_CASE)),
                    ..Default::default()
                },
                transform: Transform::from_translation(Vec3::new(
                    x as f32 * TAILLE_CASE - (LARGEUR_CARTE as f32 * TAILLE_CASE) / 2.0,
                    y as f32 * TAILLE_CASE - (HAUTEUR_CARTE as f32 * TAILLE_CASE) / 2.0,
                    0.0,
                )),
                ..Default::default()
            })
                .insert(Tuile { type_tuile: carte[y][x] });
        }
    }
}

/// Place une station sur une case vide de la map
fn placer_station(carte: &mut Vec<Vec<TypePixel>>, generateur_aleatoire: &mut StdRng) -> (usize, usize) {
    loop {
        let x = generateur_aleatoire.gen_range(0..LARGEUR_CARTE);
        let y = generateur_aleatoire.gen_range(0..HAUTEUR_CARTE);

        if carte[y][x] == TypePixel::Vide {
            carte[y][x] = TypePixel::Station;
            return (x, y);
        }
    }
}

/// Fonction limitant la taille des obstacles pour éviter des regroupements trop larges
fn limiter_taille_obstacles(carte: &mut Vec<Vec<TypePixel>>) {
    let directions = [(0, 1), (1, 0), (0, -1), (-1, 0)];

    for y in 0..HAUTEUR_CARTE {
        for x in 0..LARGEUR_CARTE {
            if carte[y][x] == TypePixel::Obstacle {
                let mut taille_obstacle = 1;

                for (dx, dy) in directions.iter() {
                    let mut nx = x as isize + dx;
                    let mut ny = y as isize + dy;

                    while nx >= 0
                        && nx < LARGEUR_CARTE as isize
                        && ny >= 0
                        && ny < HAUTEUR_CARTE as isize
                        && carte[ny as usize][nx as usize] == TypePixel::Obstacle
                    {
                        taille_obstacle += 1;
                        if taille_obstacle > MAX_TAILLE_OBSTACLE {
                            carte[ny as usize][nx as usize] = TypePixel::Vide;
                        }

                        nx += dx;
                        ny += dy;
                    }
                }
            }
        }
    }
}
