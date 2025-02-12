use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use rand::{prelude::*, SeedableRng};
use std::env;

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
struct Pixel {
    // Ce champ est utilisé lors de la création de l'entité,
    // même s'il n'est pas directement lu par la suite.
    type_pixel: TypePixel,
}

/// Ressource stockant la seed
#[derive(Resource)]
struct SeedCarte {
    seed: u64,
}

/// Ressource contenant la carte générée
#[derive(Resource)]
struct Carte {
    donnees: Vec<Vec<TypePixel>>,
    largeur: usize,
    hauteur: usize,
}

impl Carte {
    /// Retourne true si la position (x,y) est hors limites ou correspond à un obstacle.
    fn est_obstacle(&self, x: isize, y: isize) -> bool {
        if x < 0 || y < 0 || x >= self.largeur as isize || y >= self.hauteur as isize {
            return true;
        }
        self.donnees[y as usize][x as usize] == TypePixel::Obstacle
    }
}

/// Ressource pour stocker la position de la station
#[derive(Resource)]
struct StationPosition {
    x: usize,
    y: usize,
}

/// Composant pour les robots
#[derive(Component)]
struct Robot {
    x: isize,
    y: isize,
}

/// Ressource pour un Timer permettant de gérer la fréquence de déplacement
#[derive(Resource)]
struct RobotTimer {
    timer: Timer,
}

/// Ressource indiquant si les robots ont déjà été spawns
#[derive(Resource)]
struct RobotsSpawned(bool);

fn main() {
    // Récupère la seed passée en argument ou en génère une aléatoire
    let seed = obtenir_seed_depuis_arguments().unwrap_or_else(generer_seed_aleatoire);
    println!("Seed utilisée : {}", seed);

    App::new()
        .add_plugins(DefaultPlugins)
        // On insère la seed afin d’obtenir une génération reproductible
        .insert_resource(SeedCarte { seed })
        // Ajout des systèmes en startup
        .add_systems(Startup, initialiser_map)
        .add_systems(Startup, generer_map)
        .add_systems(Startup, setup_robot_timer)
        .add_systems(Startup, setup_robot_spawn_flag)
        // Les systèmes en update (les commandes startup auront déjà été appliquées)
        .add_systems(Update, spawn_robots)
        .add_systems(Update, deplacement_robots)
        .run();
}

/// Initialise la caméra dans la simulation
fn initialiser_map(mut commandes: Commands) {
    commandes.spawn(Camera2dBundle::default());
}

/// Génère la carte avec obstacles, ressources et placement de la station
fn generer_map(mut commandes: Commands, seed_carte: Res<SeedCarte>) {
    println!("Seed Actuel: {}", seed_carte.seed);

    let bruit_perlin = Perlin::new(seed_carte.seed as u32);
    let mut generateur_aleatoire = StdRng::seed_from_u64(seed_carte.seed);

    let mut carte = vec![vec![TypePixel::Vide; LARGEUR_CARTE]; HAUTEUR_CARTE];

    // Génère les obstacles à l'aide du bruit de Perlin
    for y in 0..HAUTEUR_CARTE {
        for x in 0..LARGEUR_CARTE {
            let valeur_bruit = bruit_perlin.get([x as f64 * 0.1, y as f64 * 0.1]);
            if valeur_bruit > SEUIL_OBSTACLE {
                carte[y][x] = TypePixel::Obstacle;
            }
        }
    }

    // Limite la taille des obstacles pour éviter des zones trop étendues
    limiter_taille_obstacles(&mut carte);

    // Ajoute aléatoirement des ressources sur les pixels vides
    for y in 0..HAUTEUR_CARTE {
        for x in 0..LARGEUR_CARTE {
            if carte[y][x] == TypePixel::Vide {
                carte[y][x] = match generateur_aleatoire.gen_range(0..100) {
                    0..=5 => TypePixel::Energie,           // 6% de chance
                    6..=10 => TypePixel::Minerai,           // 5% de chance
                    11..=14 => TypePixel::SiteScientifique,   // 4% de chance
                    _ => TypePixel::Vide,
                };
            }
        }
    }

    // Place la station sur une case vide
    let (pos_x, pos_y) = placer_station(&mut carte, &mut generateur_aleatoire);
    println!("Station placée en ({}, {})", pos_x, pos_y);

    // Insère la carte et la position de la station dans les ressources.
    commandes.insert_resource(Carte {
        donnees: carte.clone(),
        largeur: LARGEUR_CARTE,
        hauteur: HAUTEUR_CARTE,
    });
    commandes.insert_resource(StationPosition { x: pos_x, y: pos_y });

    // Crée les entités permettant d'afficher la carte
    for y in 0..HAUTEUR_CARTE {
        for x in 0..LARGEUR_CARTE {
            let type_pixel = carte[y][x];
            let couleur = match type_pixel {
                TypePixel::Obstacle => Color::rgb(0.2, 0.2, 0.2),
                TypePixel::Energie => Color::rgb(1.0, 1.0, 0.0),
                TypePixel::Minerai => Color::rgb(0.5, 0.3, 0.1),
                TypePixel::SiteScientifique => Color::rgb(0.0, 0.8, 0.8),
                TypePixel::Station => Color::rgb(1.0, 0.0, 0.0), // Station en rouge
                TypePixel::Vide => Color::rgb(0.8, 0.8, 0.8),
            };

            commandes
                .spawn(SpriteBundle {
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
                .insert(Pixel { type_pixel });
        }
    }
}

/// Configure un Timer pour le déplacement des robots
fn setup_robot_timer(mut commandes: Commands) {
    commandes.insert_resource(RobotTimer {
        timer: Timer::from_seconds(0.5, TimerMode::Repeating),
    });
}

/// Insère un flag indiquant que les robots n'ont pas encore été spawns
fn setup_robot_spawn_flag(mut commandes: Commands) {
    commandes.insert_resource(RobotsSpawned(false));
}

/// Système qui spawne les robots (uniquement une fois, lors du premier update)
fn spawn_robots(
    mut commandes: Commands,
    station: Res<StationPosition>,
    mut robots_spawned: ResMut<RobotsSpawned>,
) {
    // Si les robots ont déjà été spawns, on ne fait rien.
    if robots_spawned.0 {
        return;
    }

    let nb_robots = 1;
    for _ in 0..nb_robots {
        let translation = Vec3::new(
            station.x as f32 * TAILLE_CASE - (LARGEUR_CARTE as f32 * TAILLE_CASE) / 2.0,
            station.y as f32 * TAILLE_CASE - (HAUTEUR_CARTE as f32 * TAILLE_CASE) / 2.0,
            1.0, // Z pour être au-dessus de la carte
        );
        commandes.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(0.0, 1.0, 0.0), // robot en vert
                    custom_size: Some(Vec2::splat(TAILLE_CASE)),
                    ..Default::default()
                },
                transform: Transform::from_translation(translation),
                ..Default::default()
            },
            Robot {
                x: station.x as isize,
                y: station.y as isize,
            },
        ));
    }

    // On passe le flag à true afin de ne spawner les robots qu'une seule fois.
    robots_spawned.0 = true;
}

/// Système de déplacement aléatoire des robots
fn deplacement_robots(
    mut timer: ResMut<RobotTimer>,
    time: Res<Time>,
    mut query: Query<(&mut Robot, &mut Transform)>,
    carte: Res<Carte>,
) {
    // On attend que le timer soit écoulé
    if !timer.timer.tick(time.delta()).finished() {
        return;
    }

    let mut rng = rand::thread_rng();
    let directions = [
        (0, 1),   // Haut
        (0, -1),  // Bas
        (1, 0),   // Droite
        (-1, 0),  // Gauche
    ];

    for (mut robot, mut transform) in &mut query {
        let (dx, dy) = directions[rng.gen_range(0..directions.len())];

        let new_x = robot.x + dx;
        let new_y = robot.y + dy;

        // Si la nouvelle case n'est pas un obstacle (ou hors limites), on déplace le robot
        if !carte.est_obstacle(new_x, new_y) {
            robot.x = new_x;
            robot.y = new_y;
            transform.translation.x =
                new_x as f32 * TAILLE_CASE - (carte.largeur as f32 * TAILLE_CASE) / 2.0;
            transform.translation.y =
                new_y as f32 * TAILLE_CASE - (carte.hauteur as f32 * TAILLE_CASE) / 2.0;
        }
    }
}

/// Récupère une seed depuis les arguments (si présente)
fn obtenir_seed_depuis_arguments() -> Option<u64> {
    let arguments: Vec<String> = env::args().collect();
    if arguments.len() > 1 {
        arguments[1].parse::<u64>().ok()
    } else {
        None
    }
}

/// Génère une seed aléatoire
fn generer_seed_aleatoire() -> u64 {
    rand::thread_rng().gen::<u64>()
}

/// Place la station sur une case vide de la carte
fn placer_station(
    carte: &mut Vec<Vec<TypePixel>>,
    generateur_aleatoire: &mut StdRng,
) -> (usize, usize) {
    loop {
        let x = generateur_aleatoire.gen_range(0..LARGEUR_CARTE);
        let y = generateur_aleatoire.gen_range(0..HAUTEUR_CARTE);

        if carte[y][x] == TypePixel::Vide {
            carte[y][x] = TypePixel::Station;
            return (x, y);
        }
    }
}

/// Limite la taille des obstacles pour éviter des regroupements trop étendus
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
