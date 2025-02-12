use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use rand::{prelude::*, SeedableRng};
//use std::collections::HashMap;
use std::env;

// Paramètres de la carte
const LARGEUR_CARTE: usize = 50;
const HAUTEUR_CARTE: usize = 30;
const TAILLE_CASE: f32 = 20.0;

// Seuil de bruit définissant les obstacles (plus haut = plus d'obstacles)
const SEUIL_OBSTACLE: f64 = 0.5;
// Taille maximale des obstacles en pixels connectés
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

/// Enumération des états possibles du robot
#[derive(Debug)]
enum RobotState {
    Exploring,
    // Le robot mémorise la position et le type de la ressource trouvée
    Returning { resource_x: isize, resource_y: isize, resource_type: TypePixel },
}

/// Composant pour les robots
#[derive(Component)]
struct Robot {
    x: isize,
    y: isize,
    state: RobotState,
}

/// Ressource pour un Timer permettant de gérer la fréquence de déplacement
#[derive(Resource)]
struct RobotTimer {
    timer: Timer,
}

/// Ressource indiquant si les robots ont déjà été spawns
#[derive(Resource)]
struct RobotsSpawned(bool);

/// Structure représentant une découverte (une ressource découverte par un robot)
#[derive(Debug, Clone)]
struct Discovery {
    resource: TypePixel,
    x: isize,
    y: isize,
}

/// Ressource représentant le dépôt de la station (similaire à un repo Git)
#[derive(Resource, Debug)]
struct StationRepository {
    discoveries: Vec<Discovery>,
}

fn main() {
    // Récupère la seed passée en argument ou en génère une aléatoire
    let seed = obtenir_seed_depuis_arguments().unwrap_or_else(generer_seed_aleatoire);
    println!("Seed utilisée : {}", seed);

    App::new()
        .add_plugins(DefaultPlugins)
        // On insère la seed pour une génération reproductible
        .insert_resource(SeedCarte { seed })
        // Dépôt de la station qui stockera les découvertes (le "repo Git")
        .insert_resource(StationRepository { discoveries: Vec::new() })
        // Systèmes en startup
        .add_systems(Startup, initialiser_map)
        .add_systems(Startup, generer_map)
        .add_systems(Startup, setup_robot_timer)
        .add_systems(Startup, setup_robot_spawn_flag)
        // Systèmes en update
        .add_systems(Update, spawn_robots)
        .add_systems(Update, deplacement_robots)
        .run();
}

/// Initialise la caméra
fn initialiser_map(mut commandes: Commands) {
    commandes.spawn(Camera2dBundle::default());
}

/// Génère la carte avec obstacles, ressources et placement de la station
fn generer_map(mut commandes: Commands, seed_carte: Res<SeedCarte>) {
    println!("Seed Actuel: {}", seed_carte.seed);

    let bruit_perlin = Perlin::new(seed_carte.seed as u32);
    let mut generateur_aleatoire = StdRng::seed_from_u64(seed_carte.seed);

    let mut carte = vec![vec![TypePixel::Vide; LARGEUR_CARTE]; HAUTEUR_CARTE];

    // Génère les obstacles à partir du bruit de Perlin
    for y in 0..HAUTEUR_CARTE {
        for x in 0..LARGEUR_CARTE {
            let valeur_bruit = bruit_perlin.get([x as f64 * 0.1, y as f64 * 0.1]);
            if valeur_bruit > SEUIL_OBSTACLE {
                carte[y][x] = TypePixel::Obstacle;
            }
        }
    }

    // Limite la taille des obstacles
    limiter_taille_obstacles(&mut carte);

    // Ajoute aléatoirement des ressources sur les cases vides
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

    // Insère la carte et la position de la station dans les ressources
    commandes.insert_resource(Carte {
        donnees: carte.clone(),
        largeur: LARGEUR_CARTE,
        hauteur: HAUTEUR_CARTE,
    });
    commandes.insert_resource(StationPosition { x: pos_x, y: pos_y });

    // Création des entités d'affichage de la carte
    for y in 0..HAUTEUR_CARTE {
        for x in 0..LARGEUR_CARTE {
            let type_pixel = carte[y][x];
            let couleur = match type_pixel {
                TypePixel::Obstacle => Color::rgb(0.2, 0.2, 0.2),
                TypePixel::Energie => Color::rgb(1.0, 1.0, 0.0),
                TypePixel::Minerai => Color::rgb(0.5, 0.3, 0.1),
                TypePixel::SiteScientifique => Color::rgb(0.0, 0.8, 0.8),
                TypePixel::Station => Color::rgb(1.0, 0.0, 0.0),
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
        timer: Timer::from_seconds(0.1, TimerMode::Repeating),
    });
}

/// Insère un flag indiquant que les robots n'ont pas encore été spawns
fn setup_robot_spawn_flag(mut commandes: Commands) {
    commandes.insert_resource(RobotsSpawned(false));
}

/// Spawne les robots (ici, un seul robot) en les plaçant à la station
fn spawn_robots(
    mut commandes: Commands,
    station: Res<StationPosition>,
    mut robots_spawned: ResMut<RobotsSpawned>,
) {
    if robots_spawned.0 {
        return;
    }

    let nb_robots = 2;
    for _ in 0..nb_robots {
        let translation = Vec3::new(
            station.x as f32 * TAILLE_CASE - (LARGEUR_CARTE as f32 * TAILLE_CASE) / 2.0,
            station.y as f32 * TAILLE_CASE - (HAUTEUR_CARTE as f32 * TAILLE_CASE) / 2.0,
            1.0, // Pour être au-dessus de la carte
        );
        commandes.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(0.0, 1.0, 0.0), // Robot en vert
                    custom_size: Some(Vec2::splat(TAILLE_CASE)),
                    ..Default::default()
                },
                transform: Transform::from_translation(translation),
                ..Default::default()
            },
            Robot {
                x: station.x as isize,
                y: station.y as isize,
                state: RobotState::Exploring,
            },
        ));
    }

    robots_spawned.0 = true;
}

/// Système de déplacement des robots
fn deplacement_robots(
    mut timer: ResMut<RobotTimer>,
    time: Res<Time>,
    mut query: Query<(&mut Robot, &mut Transform)>,
    mut carte: ResMut<Carte>,
    station: Res<StationPosition>,
    mut repo: ResMut<StationRepository>,
) {
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

    for (mut robot, mut transform) in query.iter_mut() {
        match robot.state {
            RobotState::Exploring => {
                let (dx, dy) = directions[rng.gen_range(0..directions.len())];
                let new_x = robot.x + dx;
                let new_y = robot.y + dy;

                if !carte.est_obstacle(new_x, new_y) {
                    robot.x = new_x;
                    robot.y = new_y;
                    transform.translation.x =
                        new_x as f32 * TAILLE_CASE - (carte.largeur as f32 * TAILLE_CASE) / 2.0;
                    transform.translation.y =
                        new_y as f32 * TAILLE_CASE - (carte.hauteur as f32 * TAILLE_CASE) / 2.0;

                    // Si le robot trouve une ressource, il passe en mode Returning
                    if new_x >= 0
                        && new_y >= 0
                        && new_x < carte.largeur as isize
                        && new_y < carte.hauteur as isize
                    {
                        let tile = &mut carte.donnees[new_y as usize][new_x as usize];
                        match tile {
                            TypePixel::Energie | TypePixel::Minerai | TypePixel::SiteScientifique => {
                                println!(
                                    "Robot a trouvé une ressource {:?} en ({}, {})",
                                    tile, new_x, new_y
                                );
                                let resource_type = *tile;
                                // Marquer la ressource comme découverte pour éviter la redécouverte
                                *tile = TypePixel::Vide;
                                robot.state = RobotState::Returning {
                                    resource_x: new_x,
                                    resource_y: new_y,
                                    resource_type
                                };
                            },
                            _ => {}
                        }
                    }
                }
            },
            RobotState::Returning { resource_x, resource_y, resource_type } => {
                let target_x = station.x as isize;
                let target_y = station.y as isize;
                if robot.x == target_x && robot.y == target_y {
                    // À l'arrivée, le robot commit sa découverte dans le dépôt de la station
                    let discovery = Discovery { resource: resource_type, x: resource_x, y: resource_y };
                    commit_discovery(&mut repo, discovery);
                    robot.state = RobotState::Exploring;
                } else {
                    let dx = if target_x > robot.x { 1 } else if target_x < robot.x { -1 } else { 0 };
                    let dy = if target_y > robot.y { 1 } else if target_y < robot.y { -1 } else { 0 };

                    if dx != 0 && !carte.est_obstacle(robot.x + dx, robot.y) {
                        robot.x += dx;
                    } else if dy != 0 && !carte.est_obstacle(robot.x, robot.y + dy) {
                        robot.y += dy;
                    }
                    transform.translation.x =
                        robot.x as f32 * TAILLE_CASE - (carte.largeur as f32 * TAILLE_CASE) / 2.0;
                    transform.translation.y =
                        robot.y as f32 * TAILLE_CASE - (carte.hauteur as f32 * TAILLE_CASE) / 2.0;
                }
            },
        }
    }
}

/// Fonction de commit d'une découverte dans le dépôt de la station
/// Simule une gestion de conflits à la manière de Git :
/// - Si aucune découverte n'existe pour la position, la découverte est ajoutée.
/// - Sinon, si le type est différent, un conflit est signalé / on garde l'existant.
fn commit_discovery(repo: &mut StationRepository, discovery: Discovery) {
    if let Some(existing) = repo
        .discoveries
        .iter()
        .find(|d| d.x == discovery.x && d.y == discovery.y)
    {
        if existing.resource != discovery.resource {
            println!(
                "Conflit détecté pour la ressource en ({}, {}): {:?} vs {:?}",
                discovery.x, discovery.y, existing.resource, discovery.resource
            );
            // Pour cet exemple conservons la découverte existante.
        } else {
            println!(
                "Découverte déjà commitée pour la ressource en ({}, {})",
                discovery.x, discovery.y
            );
        }
    } else {
        repo.discoveries.push(discovery.clone());
        println!("Découverte commitée: {:?}", discovery);
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
