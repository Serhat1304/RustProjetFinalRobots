use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use rand::{prelude::*, SeedableRng};
use std::env;

// ============================
// Paramètres de la carte
// ============================
const LARGEUR_CARTE: usize = 50;
const HAUTEUR_CARTE: usize = 30;
const TAILLE_CASE: f32 = 20.0;

// Seuil du bruit de Perlin pour déterminer la présence d'obstacles
const SEUIL_OBSTACLE: f64 = 0.5;
// Taille maximale d'un groupe d'obstacles connectés
const TAILLE_MAX_OBSTACLE: usize = 5;

// ============================
// Définition des types de pixels de la carte
// ============================
#[derive(Debug, Clone, Copy, PartialEq)]
enum TypePixel {
    Vide,
    Obstacle,
    Energie,
    Minerai,
    SiteScientifique,
    Station,
}

// Composant associé à chaque pixel (pour l'affichage)
#[derive(Component)]
struct Pixel {
    type_pixel: TypePixel,
}

// ============================
// Ressources pour la génération de la carte
// ============================
#[derive(Resource)]
struct SeedCarte {
    seed: u64,
}

#[derive(Resource)]
struct Carte {
    donnees: Vec<Vec<TypePixel>>,
    largeur: usize,
    hauteur: usize,
}

impl Carte {
    /// Vérifie si la position (x, y) est hors limites ou correspond à un obstacle.
    fn est_obstacle(&self, x: isize, y: isize) -> bool {
        if x < 0 || y < 0 || x >= self.largeur as isize || y >= self.hauteur as isize {
            return true;
        }
        self.donnees[y as usize][x as usize] == TypePixel::Obstacle
    }
}

// ============================
// Ressources liées à la station
// ============================
#[derive(Resource)]
struct PositionStation {
    x: usize,
    y: usize,
}

#[derive(Resource, Debug)]
struct DepotStation {
    decouvertes: Vec<Decouverte>,
}

#[derive(Debug, Clone)]
struct Decouverte {
    resource: TypePixel,
    x: isize,
    y: isize,
}

// ============================
// Définition et comportement des robots
// ============================
#[derive(Debug)]
enum EtatRobot {
    Explorer,
    Retourner,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum RoleRobot {
    Explorateur,
    Collecteur,
}

// On modifie le composant Robot pour y ajouter un vecteur de découvertes pour les explorateurs
#[derive(Component)]
struct Robot {
    x: isize,
    y: isize,
    etat: EtatRobot,
    role: RoleRobot,
    /// Pour les explorateurs, stocke les découvertes (jusqu'à 10)
    decouvertes: Vec<Decouverte>,
    /// Pour les collecteurs, contient la ressource récupérée (lorsqu’elle est chargée)
    cargo: Option<(TypePixel, isize, isize)>,
    /// Pour les collecteurs, cible la position d'une ressource à récupérer
    cible: Option<(isize, isize)>,
}

// Ressource pour gérer la fréquence de déplacement des robots via un Timer
#[derive(Resource)]
struct MinuterieRobot {
    timer: Timer,
}

// Indique si les robots ont déjà été créés
#[derive(Resource)]
struct RobotsCrees(bool);

// ============================
// Fonction principale (point d'entrée)
// ============================
fn main() {
    let seed = obtenir_seed_depuis_arguments().unwrap_or_else(generer_seed_aleatoire);
    println!("Seed utilisée : {}", seed);

    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(SeedCarte { seed })
        .insert_resource(DepotStation {
            decouvertes: Vec::new(),
        })
        .add_systems(Startup, initialiser_camera)
        .add_systems(Startup, generer_carte)
        .add_systems(Startup, configurer_minuterie_robot)
        .add_systems(Startup, initialiser_robots_crees)
        .add_systems(Update, creer_robots)
        .add_systems(Update, deplacer_robots)
        .add_systems(Update, synchroniser_pixels_carte.after(deplacer_robots))
        .run();
}

// ============================
// Systèmes d'initialisation
// ============================
fn initialiser_camera(mut commandes: Commands) {
    commandes.spawn(Camera2dBundle::default());
}

fn generer_carte(mut commandes: Commands, seed_carte: Res<SeedCarte>) {
    println!("Seed Actuel: {}", seed_carte.seed);

    let bruit_perlin = Perlin::new(seed_carte.seed as u32);
    let mut generateur_aleatoire = StdRng::seed_from_u64(seed_carte.seed);

    let mut carte = vec![vec![TypePixel::Vide; LARGEUR_CARTE]; HAUTEUR_CARTE];

    for y in 0..HAUTEUR_CARTE {
        for x in 0..LARGEUR_CARTE {
            let valeur_bruit = bruit_perlin.get([x as f64 * 0.1, y as f64 * 0.1]);
            if valeur_bruit > SEUIL_OBSTACLE {
                carte[y][x] = TypePixel::Obstacle;
            }
        }
    }

    limiter_taille_obstacles(&mut carte);

    for y in 0..HAUTEUR_CARTE {
        for x in 0..LARGEUR_CARTE {
            if carte[y][x] == TypePixel::Vide {
                carte[y][x] = match generateur_aleatoire.gen_range(0..100) {
                    0..=5   => TypePixel::Energie,
                    6..=10  => TypePixel::Minerai,
                    11..=14 => TypePixel::SiteScientifique,
                    _       => TypePixel::Vide,
                };
            }
        }
    }

    let (pos_x, pos_y) = placer_station(&mut carte, &mut generateur_aleatoire);
    println!("Station placée en ({}, {})", pos_x, pos_y);

    commandes.insert_resource(Carte {
        donnees: carte.clone(),
        largeur: LARGEUR_CARTE,
        hauteur: HAUTEUR_CARTE,
    });
    commandes.insert_resource(PositionStation { x: pos_x, y: pos_y });

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

            let translation = Vec3::new(
                x as f32 * TAILLE_CASE - (LARGEUR_CARTE as f32 * TAILLE_CASE) / 2.0,
                y as f32 * TAILLE_CASE - (HAUTEUR_CARTE as f32 * TAILLE_CASE) / 2.0,
                0.0,
            );

            commandes
                .spawn(SpriteBundle {
                    sprite: Sprite {
                        color: couleur,
                        custom_size: Some(Vec2::splat(TAILLE_CASE)),
                        ..Default::default()
                    },
                    transform: Transform::from_translation(translation),
                    ..Default::default()
                })
                .insert(Pixel { type_pixel });
        }
    }
}

fn configurer_minuterie_robot(mut commandes: Commands) {
    commandes.insert_resource(MinuterieRobot {
        timer: Timer::from_seconds(0.9, TimerMode::Repeating),
    });
}

fn initialiser_robots_crees(mut commandes: Commands) {
    commandes.insert_resource(RobotsCrees(false));
}

// ============================
// Création et déplacement des robots
// ============================
fn creer_robots(
    mut commandes: Commands,
    station: Res<PositionStation>,
    mut robots_crees: ResMut<RobotsCrees>,
) {
    if robots_crees.0 {
        return;
    }

    let nb_explorateurs = 3;
    let nb_collecteurs = 1;

    // Création des explorateurs (on initialise le vecteur "decouvertes" vide)
    for _ in 0..nb_explorateurs {
        let translation = Vec3::new(
            station.x as f32 * TAILLE_CASE - (LARGEUR_CARTE as f32 * TAILLE_CASE) / 2.0,
            station.y as f32 * TAILLE_CASE - (HAUTEUR_CARTE as f32 * TAILLE_CASE) / 2.0,
            1.0,
        );
        commandes.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(0.0, 1.0, 0.0), // Vert
                    custom_size: Some(Vec2::splat(TAILLE_CASE)),
                    ..Default::default()
                },
                transform: Transform::from_translation(translation),
                ..Default::default()
            },
            Robot {
                x: station.x as isize,
                y: station.y as isize,
                etat: EtatRobot::Explorer,
                role: RoleRobot::Explorateur,
                decouvertes: Vec::new(),
                cargo: None,
                cible: None,
            },
        ));
    }

    // Création des collecteurs
    for _ in 0..nb_collecteurs {
        let translation = Vec3::new(
            station.x as f32 * TAILLE_CASE - (LARGEUR_CARTE as f32 * TAILLE_CASE) / 2.0,
            station.y as f32 * TAILLE_CASE - (HAUTEUR_CARTE as f32 * TAILLE_CASE) / 2.0,
            1.0,
        );
        commandes.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(0.0, 0.0, 1.0), // Bleu
                    custom_size: Some(Vec2::splat(TAILLE_CASE)),
                    ..Default::default()
                },
                transform: Transform::from_translation(translation),
                ..Default::default()
            },
            Robot {
                x: station.x as isize,
                y: station.y as isize,
                etat: EtatRobot::Explorer,
                role: RoleRobot::Collecteur,
                decouvertes: Vec::new(), // non utilisé pour les collecteurs
                cargo: None,
                cible: None,
            },
        ));
    }

    robots_crees.0 = true;
}

fn deplacer_robots(
    mut minuterie: ResMut<MinuterieRobot>,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Robot, &mut Transform)>,
    mut carte: ResMut<Carte>,
    station: Res<PositionStation>,
    mut depot: ResMut<DepotStation>,
) {
    if !minuterie.timer.tick(time.delta()).finished() {
        return;
    }

    let mut rng = rand::thread_rng();
    let directions = [(0, 1), (0, -1), (1, 0), (-1, 0)];

    for (_entity, mut robot, mut transform) in query.iter_mut() {
        match robot.role {
            RoleRobot::Explorateur => {
                match robot.etat {
                    // Mode exploration : déplacement aléatoire et accumulation de découvertes
                    EtatRobot::Explorer => {
                        let (dx, dy) = directions[rng.gen_range(0..directions.len())];
                        let nouvelle_x = robot.x + dx;
                        let nouvelle_y = robot.y + dy;

                        if !carte.est_obstacle(nouvelle_x, nouvelle_y) {
                            robot.x = nouvelle_x;
                            robot.y = nouvelle_y;
                            transform.translation.x = nouvelle_x as f32 * TAILLE_CASE - (carte.largeur as f32 * TAILLE_CASE) / 2.0;
                            transform.translation.y = nouvelle_y as f32 * TAILLE_CASE - (carte.hauteur as f32 * TAILLE_CASE) / 2.0;

                            // Si la case contient une ressource (énergie ou minerai)
                            if nouvelle_x >= 0 && nouvelle_y >= 0 && nouvelle_x < carte.largeur as isize && nouvelle_y < carte.hauteur as isize {
                                let tuile = carte.donnees[nouvelle_y as usize][nouvelle_x as usize];
                                if (tuile == TypePixel::Energie || tuile == TypePixel::Minerai) {
                                    // Ajoute la découverte si elle n'a pas déjà été enregistrée par ce robot
                                    let deja_trouve = robot.decouvertes.iter().any(|d| d.x == nouvelle_x && d.y == nouvelle_y);
                                    if !deja_trouve {
                                        println!("Explorateur détecte la ressource {:?} en ({}, {})", tuile, nouvelle_x, nouvelle_y);
                                        robot.decouvertes.push(Decouverte { resource: tuile, x: nouvelle_x, y: nouvelle_y });
                                        if robot.decouvertes.len() >= 10 {
                                            robot.etat = EtatRobot::Retourner;
                                        }
                                    }
                                }
                            }
                        }
                    },
                    // Mode retour : le robot se dirige vers la station pour "commiter" ses découvertes
                    EtatRobot::Retourner => {
                        let cible_x = station.x as isize;
                        let cible_y = station.y as isize;
                        if robot.x == cible_x && robot.y == cible_y {
                            for decouverte in &robot.decouvertes {
                                if decouverte.resource == TypePixel::Energie || decouverte.resource == TypePixel::Minerai {
                                    enregistrer_decouverte(&mut depot, decouverte.clone());
                                }
                            }
                            robot.decouvertes.clear();
                            robot.etat = EtatRobot::Explorer;
                        } else {
                            let dx = if cible_x > robot.x { 1 } else if cible_x < robot.x { -1 } else { 0 };
                            let dy = if cible_y > robot.y { 1 } else if cible_y < robot.y { -1 } else { 0 };
                            if dx != 0 && !carte.est_obstacle(robot.x + dx, robot.y) {
                                robot.x += dx;
                            } else if dy != 0 && !carte.est_obstacle(robot.x, robot.y + dy) {
                                robot.y += dy;
                            }
                            transform.translation.x = robot.x as f32 * TAILLE_CASE - (carte.largeur as f32 * TAILLE_CASE) / 2.0;
                            transform.translation.y = robot.y as f32 * TAILLE_CASE - (carte.hauteur as f32 * TAILLE_CASE) / 2.0;
                        }
                    },
                }
            },
            RoleRobot::Collecteur => {
                match robot.etat {
                    EtatRobot::Explorer => {
                        // Si le collecteur est à la station et n'a pas de cible, on lui assigne une ressource en retirant la découverte du dépôt
                        if robot.x == station.x as isize && robot.y == station.y as isize {
                            if robot.cible.is_none() {
                                if let Some(index) = depot.decouvertes.iter().position(|d|
                                    (d.resource == TypePixel::Energie || d.resource == TypePixel::Minerai) &&
                                        carte.donnees[d.y as usize][d.x as usize] == d.resource
                                ) {
                                    let decouverte = depot.decouvertes.remove(index);
                                    robot.cible = Some((decouverte.x, decouverte.y));
                                    println!("Collecteur part avec pour cible ({}, {})", decouverte.x, decouverte.y);
                                }
                            }
                        }
                        if let Some((cible_x, cible_y)) = robot.cible {
                            let step_dx = if cible_x > robot.x { 1 } else if cible_x < robot.x { -1 } else { 0 };
                            let step_dy = if cible_y > robot.y { 1 } else if cible_y < robot.y { -1 } else { 0 };
                            if !carte.est_obstacle(robot.x + step_dx, robot.y + step_dy) {
                                robot.x += step_dx;
                                robot.y += step_dy;
                                transform.translation.x = robot.x as f32 * TAILLE_CASE - (carte.largeur as f32 * TAILLE_CASE) / 2.0;
                                transform.translation.y = robot.y as f32 * TAILLE_CASE - (carte.hauteur as f32 * TAILLE_CASE) / 2.0;
                            }
                            if robot.x == cible_x && robot.y == cible_y {
                                let tuile = &mut carte.donnees[cible_y as usize][cible_x as usize];
                                if *tuile == TypePixel::Energie || *tuile == TypePixel::Minerai {
                                    println!("Collecteur récupère la ressource {:?} en ({}, {})", *tuile, cible_x, cible_y);
                                    let resource_type = *tuile;
                                    *tuile = TypePixel::Vide;
                                    robot.cargo = Some((resource_type, cible_x, cible_y));
                                    robot.cible = None;
                                    robot.etat = EtatRobot::Retourner;
                                } else {
                                    robot.cible = None;
                                }
                            }
                        }
                    },
                    EtatRobot::Retourner => {
                        let cible_x = station.x as isize;
                        let cible_y = station.y as isize;
                        if robot.x == cible_x && robot.y == cible_y {
                            if let Some((resource, res_x, res_y)) = robot.cargo.take() {
                                println!("Collecteur dépose la ressource {:?} collectée de ({}, {}) à la station", resource, res_x, res_y);
                            }
                            robot.etat = EtatRobot::Explorer;
                        } else {
                            let dx = if cible_x > robot.x { 1 } else if cible_x < robot.x { -1 } else { 0 };
                            let dy = if cible_y > robot.y { 1 } else if cible_y < robot.y { -1 } else { 0 };
                            if dx != 0 && !carte.est_obstacle(robot.x + dx, robot.y) {
                                robot.x += dx;
                            } else if dy != 0 && !carte.est_obstacle(robot.x, robot.y + dy) {
                                robot.y += dy;
                            }
                            transform.translation.x = robot.x as f32 * TAILLE_CASE - (carte.largeur as f32 * TAILLE_CASE) / 2.0;
                            transform.translation.y = robot.y as f32 * TAILLE_CASE - (carte.hauteur as f32 * TAILLE_CASE) / 2.0;
                        }
                    },
                }
            },
        }
    }
}

fn synchroniser_pixels_carte(
    carte: Res<Carte>,
    mut query: Query<(&mut Pixel, &mut Sprite, &Transform)>,
) {
    for (mut pixel, mut sprite, transform) in query.iter_mut() {
        let tile_x = ((transform.translation.x + (carte.largeur as f32 * TAILLE_CASE) / 2.0) / TAILLE_CASE)
            .round() as usize;
        let tile_y = ((transform.translation.y + (carte.hauteur as f32 * TAILLE_CASE) / 2.0) / TAILLE_CASE)
            .round() as usize;
        if tile_x < carte.largeur && tile_y < carte.hauteur {
            let nouveau_type = carte.donnees[tile_y][tile_x];
            if pixel.type_pixel != nouveau_type {
                pixel.type_pixel = nouveau_type;
                sprite.color = match nouveau_type {
                    TypePixel::Obstacle => Color::rgb(0.2, 0.2, 0.2),
                    TypePixel::Energie => Color::rgb(1.0, 1.0, 0.0),
                    TypePixel::Minerai => Color::rgb(0.5, 0.3, 0.1),
                    TypePixel::SiteScientifique => Color::rgb(0.0, 0.8, 0.8),
                    TypePixel::Station => Color::rgb(1.0, 0.0, 0.0),
                    TypePixel::Vide => Color::rgb(0.8, 0.8, 0.8),
                };
            }
        }
    }
}

// ============================
// Fonctions utilitaires
// ============================
fn enregistrer_decouverte(depot: &mut DepotStation, decouverte: Decouverte) {
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

fn obtenir_seed_depuis_arguments() -> Option<u64> {
    let arguments: Vec<String> = env::args().collect();
    if arguments.len() > 1 {
        arguments[1].parse::<u64>().ok()
    } else {
        None
    }
}

fn generer_seed_aleatoire() -> u64 {
    rand::thread_rng().gen::<u64>()
}

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
                        if taille_obstacle > TAILLE_MAX_OBSTACLE {
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
