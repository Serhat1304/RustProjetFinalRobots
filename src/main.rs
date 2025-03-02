use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use rand::{prelude::*, SeedableRng};
use std::collections::{HashSet, VecDeque, HashMap};
use std::env;

// ============================
// PARAMÈTRES DE LA CARTE
// ============================
const LARGEUR_CARTE: usize = 50;
const HAUTEUR_CARTE: usize = 30;
const TAILLE_CASE: f32 = 20.0;

// Seuil du bruit de Perlin pour déterminer la présence d'obstacles
const SEUIL_OBSTACLE: f64 = 0.5;
// Taille maximale d'un groupe d'obstacles connectés
const TAILLE_MAX_OBSTACLE: usize = 5;

// ============================
// DÉFINITION DES TYPES DE PIXELS DE LA CARTE
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
// RESSOURCES POUR LA GÉNÉRATION DE LA CARTE
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
// RESSOURCES LIÉES À LA STATION
// ============================
#[derive(Resource)]
struct PositionStation {
    x: usize,
    y: usize,
}

#[derive(Resource, Debug)]
struct DepotStation {
    decouvertes: Vec<Decouverte>,
    stock_energie: u32,
    stock_minerai: u32,
}

#[derive(Debug, Clone)]
struct Decouverte {
    resource: TypePixel,
    x: isize,
    y: isize,
}

// ============================
// DÉFINITION DES MODULES SPÉCIALISÉS POUR LES ROBOTS
// ============================
#[derive(Debug, Clone, Copy, PartialEq)]
enum ModuleRobot {
    AnalyseChimique,         // Pour les collecteurs récupérant l'énergie
    Forage,                  // Pour les collecteurs récupérant des minerais
    ImagerieHauteResolution, // Pour les explorateurs
}

// ============================
// DÉFINITION ET COMPORTEMENT DES ROBOTS
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

#[derive(Component)]
struct Robot {
    x: isize,
    y: isize,
    etat: EtatRobot,
    role: RoleRobot,
    /// Pour les explorateurs : stocke les découvertes (jusqu'à 10)
    decouvertes: Vec<Decouverte>,
    /// Pour les collecteurs : stocke la ressource récupérée (lorsqu’elle est chargée)
    cargo: Option<(TypePixel, isize, isize)>,
    /// Pour les collecteurs : cible la position d'une ressource à récupérer
    cible: Option<(isize, isize)>,
    /// Pour les explorateurs : ensemble des cases déjà visitées
    visites: HashSet<(isize, isize)>,
    /// Liste des modules spécialisés installés sur le robot
    modules: Vec<ModuleRobot>,
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
// FONCTION PRINCIPALE (POINT D'ENTRÉE)
// ============================
fn main() {
    let seed = obtenir_seed_depuis_arguments().unwrap_or_else(generer_seed_aleatoire);
    println!("Seed utilisée : {}", seed);

    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(SeedCarte { seed })
        .insert_resource(DepotStation {
            decouvertes: Vec::new(),
            stock_energie: 0,
            stock_minerai: 0,
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
// SYSTÈMES D'INITIALISATION
// ============================
fn initialiser_camera(mut commandes: Commands) {
    commandes.spawn(Camera2dBundle::default());
}

fn generer_carte(mut commandes: Commands, seed_carte: Res<SeedCarte>) {
    println!("Seed Actuel: {}", seed_carte.seed);

    let bruit_perlin = Perlin::new(seed_carte.seed as u32);
    let mut generateur_aleatoire = rand::rngs::StdRng::seed_from_u64(seed_carte.seed);

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

    // Affichage de la carte avec une valeur z différente pour la station
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

            let z_coord = if type_pixel == TypePixel::Station { 2.0 } else { 0.0 };

            let translation = Vec3::new(
                x as f32 * TAILLE_CASE - (LARGEUR_CARTE as f32 * TAILLE_CASE) / 2.0,
                y as f32 * TAILLE_CASE - (HAUTEUR_CARTE as f32 * TAILLE_CASE) / 2.0,
                z_coord,
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
        timer: Timer::from_seconds(0.2, TimerMode::Repeating),
    });
}

fn initialiser_robots_crees(mut commandes: Commands) {
    commandes.insert_resource(RobotsCrees(false));
}

// ============================
// CRÉATION ET DÉPLACEMENT DES ROBOTS
// ============================
fn creer_robots(
    mut commandes: Commands,
    station: Res<PositionStation>,
    mut robots_crees: ResMut<RobotsCrees>,
) {
    if robots_crees.0 {
        return;
    }

    // Création des explorateurs spécialisés en imagerie haute résolution
    let nb_explorateurs = 5;
    for _ in 0..nb_explorateurs {
        let translation = Vec3::new(
            station.x as f32 * TAILLE_CASE - (LARGEUR_CARTE as f32 * TAILLE_CASE) / 2.0,
            station.y as f32 * TAILLE_CASE - (HAUTEUR_CARTE as f32 * TAILLE_CASE) / 2.0,
            1.0,
        );
        commandes.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(0.0, 1.0, 0.0), // Vert pour les explorateurs
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
                visites: HashSet::new(),
                modules: vec![ModuleRobot::ImagerieHauteResolution],
            },
        ));
    }

    // Création des collecteurs spécialisés
    let nb_collecteurs_analyse = 1;
    let nb_collecteurs_forage = 1;

    for _ in 0..nb_collecteurs_analyse {
        let translation = Vec3::new(
            station.x as f32 * TAILLE_CASE - (LARGEUR_CARTE as f32 * TAILLE_CASE) / 2.0,
            station.y as f32 * TAILLE_CASE - (HAUTEUR_CARTE as f32 * TAILLE_CASE) / 2.0,
            1.0,
        );
        commandes.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(0.0, 0.5, 1.0),
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
                decouvertes: Vec::new(),
                cargo: None,
                cible: None,
                visites: HashSet::new(),
                modules: vec![ModuleRobot::AnalyseChimique],
            },
        ));
    }

    for _ in 0..nb_collecteurs_forage {
        let translation = Vec3::new(
            station.x as f32 * TAILLE_CASE - (LARGEUR_CARTE as f32 * TAILLE_CASE) / 2.0,
            station.y as f32 * TAILLE_CASE - (HAUTEUR_CARTE as f32 * TAILLE_CASE) / 2.0,
            1.0,
        );
        commandes.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(0.5, 0.0, 1.0),
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
                decouvertes: Vec::new(),
                cargo: None,
                cible: None,
                visites: HashSet::new(),
                modules: vec![ModuleRobot::Forage],
            },
        ));
    }

    robots_crees.0 = true;
}

fn deplacer_robots(
    mut commandes: Commands,
    mut minuterie: ResMut<MinuterieRobot>,
    temps: Res<Time>,
    mut requete: Query<(Entity, &mut Robot, &mut Transform)>,
    mut carte: ResMut<Carte>,
    station: Res<PositionStation>,
    mut depot: ResMut<DepotStation>,
) {
    if !minuterie.timer.tick(temps.delta()).finished() {
        return;
    }

    let mut rng = rand::thread_rng();
    let directions = [(0, 1), (0, -1), (1, 0), (-1, 0)];

    for (_entite, mut robot, mut transform) in requete.iter_mut() {
        match robot.role {
            RoleRobot::Explorateur => {
                match robot.etat {
                    EtatRobot::Explorer => {
                        let position_actuelle = (robot.x, robot.y);
                        robot.visites.insert(position_actuelle);

                        // Recherche des déplacements possibles
                        let deplacements_possibles: Vec<(isize, isize)> = directions.iter()
                            .map(|(dx, dy)| (robot.x + dx, robot.y + dy))
                            .filter(|(nx, ny)| {
                                *nx >= 0 &&
                                    *ny >= 0 &&
                                    *nx < carte.largeur as isize &&
                                    *ny < carte.hauteur as isize &&
                                    !carte.est_obstacle(*nx, *ny)
                            })
                            .collect();

                        // Privilégier les cases non visitées
                        let deplacements_non_visites: Vec<(isize, isize)> = deplacements_possibles
                            .iter()
                            .cloned()
                            .filter(|pos| !robot.visites.contains(pos))
                            .collect();

                        let (nouveau_x, nouveau_y) = if !deplacements_non_visites.is_empty() {
                            deplacements_non_visites[rng.gen_range(0..deplacements_non_visites.len())]
                        } else if !deplacements_possibles.is_empty() {
                            deplacements_possibles[rng.gen_range(0..deplacements_possibles.len())]
                        } else {
                            (robot.x, robot.y)
                        };

                        robot.x = nouveau_x;
                        robot.y = nouveau_y;
                        transform.translation.x = nouveau_x as f32 * TAILLE_CASE - (carte.largeur as f32 * TAILLE_CASE) / 2.0;
                        transform.translation.y = nouveau_y as f32 * TAILLE_CASE - (carte.hauteur as f32 * TAILLE_CASE) / 2.0;

                        // Détection des ressources
                        if nouveau_x >= 0 && nouveau_y >= 0 &&
                            nouveau_x < carte.largeur as isize &&
                            nouveau_y < carte.hauteur as isize
                        {
                            let tuile = carte.donnees[nouveau_y as usize][nouveau_x as usize];
                            if tuile == TypePixel::Energie || tuile == TypePixel::Minerai {
                                let deja_trouve = robot.decouvertes.iter().any(|d| d.x == nouveau_x && d.y == nouveau_y);
                                if !deja_trouve {
                                    println!("Explorateur détecte la ressource {:?} en ({}, {})", tuile, nouveau_x, nouveau_y);
                                    robot.decouvertes.push(Decouverte { resource: tuile, x: nouveau_x, y: nouveau_y });
                                    if robot.decouvertes.len() >= 2 {
                                        robot.etat = EtatRobot::Retourner;
                                    }
                                }
                            }
                        }
                    },
                    EtatRobot::Retourner => {
                        let cible = (station.x as isize, station.y as isize);
                        if robot.x == cible.0 && robot.y == cible.1 {
                            for dec in &robot.decouvertes {
                                if dec.resource == TypePixel::Energie || dec.resource == TypePixel::Minerai {
                                    enregistrer_decouverte(&mut depot, dec.clone());
                                }
                            }
                            robot.decouvertes.clear();
                            robot.etat = EtatRobot::Explorer;
                        } else {
                            if let Some(chemin) = calculer_chemin_bfs(&carte, (robot.x, robot.y), cible) {
                                if chemin.len() > 1 {
                                    let (nx, ny) = chemin[1];
                                    robot.x = nx;
                                    robot.y = ny;
                                    transform.translation.x = nx as f32 * TAILLE_CASE - (carte.largeur as f32 * TAILLE_CASE) / 2.0;
                                    transform.translation.y = ny as f32 * TAILLE_CASE - (carte.hauteur as f32 * TAILLE_CASE) / 2.0;
                                }
                            } else {
                                println!("Explorateur bloqué ou aucun chemin vers la station");
                            }
                        }
                    },
                }
            },
            RoleRobot::Collecteur => {
                match robot.etat {
                    EtatRobot::Explorer => {
                        let resource_filtre = if robot.modules.contains(&ModuleRobot::AnalyseChimique) {
                            TypePixel::Energie
                        } else if robot.modules.contains(&ModuleRobot::Forage) {
                            TypePixel::Minerai
                        } else {
                            TypePixel::Vide
                        };

                        if robot.x == station.x as isize && robot.y == station.y as isize {
                            if robot.cible.is_none() {
                                if let Some(index) = depot.decouvertes.iter().position(|d|
                                    d.resource == resource_filtre &&
                                        carte.donnees[d.y as usize][d.x as usize] == d.resource
                                ) {
                                    let decouverte = depot.decouvertes.remove(index);
                                    robot.cible = Some((decouverte.x, decouverte.y));
                                    println!("Collecteur {:?} part avec pour cible ({}, {})", robot.modules, decouverte.x, decouverte.y);
                                }
                            }
                        }

                        if let Some((cx, cy)) = robot.cible {
                            if let Some(chemin) = calculer_chemin_bfs(&carte, (robot.x, robot.y), (cx, cy)) {
                                if chemin.len() > 1 {
                                    let (nx, ny) = chemin[1];
                                    robot.x = nx;
                                    robot.y = ny;
                                    transform.translation.x = nx as f32 * TAILLE_CASE - (carte.largeur as f32 * TAILLE_CASE) / 2.0;
                                    transform.translation.y = ny as f32 * TAILLE_CASE - (carte.hauteur as f32 * TAILLE_CASE) / 2.0;
                                }
                                if robot.x == cx && robot.y == cy {
                                    let tuile = &mut carte.donnees[cy as usize][cx as usize];
                                    if *tuile == resource_filtre {
                                        println!("Collecteur {:?} récupère la ressource {:?} en ({}, {})", robot.modules, *tuile, cx, cy);
                                        let resource_type = *tuile;
                                        *tuile = TypePixel::Vide;
                                        robot.cargo = Some((resource_type, cx, cy));
                                        robot.cible = None;
                                        robot.etat = EtatRobot::Retourner;
                                    } else {
                                        robot.cible = None;
                                        robot.etat = EtatRobot::Retourner;
                                    }
                                }
                            } else {
                                println!("Collecteur ne peut accéder à ({}, {}) : blocage. Ressource abandonnée.", cx, cy);
                                robot.cible = None;
                                robot.etat = EtatRobot::Retourner;
                            }
                        }
                    },
                    EtatRobot::Retourner => {
                        let cible = (station.x as isize, station.y as isize);
                        if robot.x == cible.0 && robot.y == cible.1 {
                            if let Some((resource, res_x, res_y)) = robot.cargo.take() {
                                println!("Collecteur dépose la ressource {:?} collectée de ({}, {}) à la station", resource, res_x, res_y);
                                match resource {
                                    TypePixel::Energie => {
                                        depot.stock_energie += 1;
                                        println!("Stock d'énergie à la station: {}", depot.stock_energie);
                                        if depot.stock_energie >= 3 {
                                            depot.stock_energie -= 3;
                                            println!("3 énergies accumulées --> Création d'un nouveau collecteur spécialisé en minerais.");
                                            creer_collecteur(&mut commandes, &station, ModuleRobot::Forage);
                                        }
                                    }
                                    TypePixel::Minerai => {
                                        depot.stock_minerai += 1;
                                        println!("Stock de minerais à la station: {}", depot.stock_minerai);
                                        if depot.stock_minerai >= 3 {
                                            depot.stock_minerai -= 3;
                                            println!("3 minerais accumulés --> Création d'un nouveau collecteur spécialisé en énergie.");
                                            creer_collecteur(&mut commandes, &station, ModuleRobot::AnalyseChimique);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            robot.etat = EtatRobot::Explorer;
                        } else {
                            if let Some(chemin) = calculer_chemin_bfs(&carte, (robot.x, robot.y), cible) {
                                if chemin.len() > 1 {
                                    let (nx, ny) = chemin[1];
                                    robot.x = nx;
                                    robot.y = ny;
                                    transform.translation.x = nx as f32 * TAILLE_CASE - (carte.largeur as f32 * TAILLE_CASE) / 2.0;
                                    transform.translation.y = ny as f32 * TAILLE_CASE - (carte.hauteur as f32 * TAILLE_CASE) / 2.0;
                                }
                            } else {
                                println!("Collecteur bloqué : aucun chemin vers la station");
                            }
                        }
                    },
                }
            },
        }
    }
}

fn synchroniser_pixels_carte(
    carte: Res<Carte>,
    mut requete: Query<(&mut Pixel, &mut Sprite, &Transform)>,
) {
    for (mut pixel, mut sprite, transform) in requete.iter_mut() {
        let tile_x = ((transform.translation.x + (carte.largeur as f32 * TAILLE_CASE) / 2.0) / TAILLE_CASE).round() as usize;
        let tile_y = ((transform.translation.y + (carte.hauteur as f32 * TAILLE_CASE) / 2.0) / TAILLE_CASE).round() as usize;
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
// FONCTIONS UTILITAIRES
// ============================
fn enregistrer_decouverte(depot: &mut DepotStation, decouverte: Decouverte) {
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

fn placer_station(carte: &mut Vec<Vec<TypePixel>>, generateur_aleatoire: &mut rand::rngs::StdRng) -> (usize, usize) {
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
                    while nx >= 0 && nx < LARGEUR_CARTE as isize && ny >= 0 && ny < HAUTEUR_CARTE as isize && carte[ny as usize][nx as usize] == TypePixel::Obstacle {
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

/// Calcule un chemin de départ à arrivée via BFS.
/// Retourne None si aucun chemin n'existe, ou Some(chemin) si un chemin est trouvé.
fn calculer_chemin_bfs(carte: &Carte, depart: (isize, isize), arrivee: (isize, isize)) -> Option<Vec<(isize, isize)>> {
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

/// Crée un nouveau collecteur à la station selon le module spécifié.
fn creer_collecteur(commandes: &mut Commands, station: &PositionStation, module: ModuleRobot) {
    let (couleur, modules) = match module {
        ModuleRobot::Forage => (Color::rgb(0.5, 0.0, 1.0), vec![ModuleRobot::Forage]),
        ModuleRobot::AnalyseChimique => (Color::rgb(0.0, 0.5, 1.0), vec![ModuleRobot::AnalyseChimique]),
        _ => (Color::WHITE, vec![]),
    };
    let translation = Vec3::new(
        station.x as f32 * TAILLE_CASE - (LARGEUR_CARTE as f32 * TAILLE_CASE) / 2.0,
        station.y as f32 * TAILLE_CASE - (HAUTEUR_CARTE as f32 * TAILLE_CASE) / 2.0,
        1.0,
    );
    commandes.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: couleur,
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
            decouvertes: Vec::new(),
            cargo: None,
            cible: None,
            visites: HashSet::new(),
            modules,
        },
    ));
}
