use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use futures_lite::future;
use std::collections::HashSet;
use rand::Rng;
use crate::carte::{TAILLE_CASE, LARGEUR_CARTE, HAUTEUR_CARTE, Carte, TypePixel, PositionStation, DepotStation, Decouverte};
use crate::utils::{calculer_chemin_bfs, enregistrer_decouverte, Evenement, Evenements};

/// Modules spécialisés installés sur les robots.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModuleRobot {
    AnalyseChimique,         // Pour collecter l'énergie
    Forage,                  // Pour collecter les minerais
    ImagerieHauteResolution, // Pour les explorateurs
}

/// État d'un robot : en exploration ou en train de retourner à la station.
#[derive(Debug)]
pub enum EtatRobot {
    Explorer,
    Retourner,
}

/// Rôle d'un robot : explorateur ou collecteur.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RoleRobot {
    Explorateur,
    Collecteur,
}

/// Structure représentant un robot dans le jeu.
#[derive(Component)]
pub struct Robot {
    pub x: isize,
    pub y: isize,
    pub etat: EtatRobot,
    pub role: RoleRobot,
    pub decouvertes: Vec<Decouverte>,
    pub cargo: Option<(TypePixel, isize, isize)>,
    pub cible: Option<(isize, isize)>,
    pub visites: HashSet<(isize, isize)>,
    pub modules: Vec<ModuleRobot>,
}

/// Système de création des robots lors de l'initialisation.
pub fn creer_robots(
    mut commandes: Commands,
    station: Res<PositionStation>,
    mut robots_crees: ResMut<crate::systemes::RobotsCrees>,
) {
    if robots_crees.0 {
        return;
    }

    // Création de 5 explorateurs (couleur verte)
    let nb_explorateurs = 3;
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
                visites: HashSet::new(),
                modules: vec![ModuleRobot::ImagerieHauteResolution],
            },
        ));
    }

    // Création des collecteurs spécialisés par défaut
    let nb_collecteurs_analyse = 1;
    let nb_collecteurs_forage = 1;

    // Collecteurs pour l'énergie (couleur bleue)
    for _ in 0..nb_collecteurs_analyse {
        let translation = Vec3::new(
            station.x as f32 * TAILLE_CASE - (LARGEUR_CARTE as f32 * TAILLE_CASE) / 2.0,
            station.y as f32 * TAILLE_CASE - (HAUTEUR_CARTE as f32 * TAILLE_CASE) / 2.0,
            1.0,
        );
        commandes.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(0.0, 0.5, 1.0), // Bleu
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

    // Collecteurs pour les minerais (couleur violette)
    for _ in 0..nb_collecteurs_forage {
        let translation = Vec3::new(
            station.x as f32 * TAILLE_CASE - (LARGEUR_CARTE as f32 * TAILLE_CASE) / 2.0,
            station.y as f32 * TAILLE_CASE - (HAUTEUR_CARTE as f32 * TAILLE_CASE) / 2.0,
            1.0,
        );
        commandes.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(0.5, 0.0, 1.0), // Violet
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

/// Système de déplacement des robots.
/// Le calcul du chemin est effectué via AsyncComputeTaskPool pour éviter de bloquer le thread principal.
/// Des événements sont enregistrés pour l'event sourcing.
pub fn deplacer_robots(
    mut commandes: Commands,
    mut minuterie: ResMut<crate::systemes::MinuterieRobot>,
    temps: Res<Time>,
    mut requete: Query<(Entity, &mut Robot, &mut Transform)>,
    mut carte: ResMut<Carte>,
    station: Res<PositionStation>,
    mut depot: ResMut<DepotStation>,
    mut evenements: ResMut<Evenements>,
) {
    if !minuterie.timer.tick(temps.delta()).finished() {
        return;
    }

    let task_pool = AsyncComputeTaskPool::get();
    let mut rng = rand::thread_rng();
    let directions = [(0, 1), (0, -1), (1, 0), (-1, 0)];

    for (entity, mut robot, mut transform) in requete.iter_mut() {
        match robot.role {
            RoleRobot::Explorateur => {
                match robot.etat {
                    EtatRobot::Explorer => {
                        let position_actuelle = (robot.x, robot.y);
                        robot.visites.insert(position_actuelle);

                        let deplacements_possibles: Vec<(isize, isize)> = directions
                            .iter()
                            .map(|(dx, dy)| (robot.x + dx, robot.y + dy))
                            .filter(|(nx, ny)| {
                                *nx >= 0 && *ny >= 0 &&
                                    *nx < carte.largeur as isize &&
                                    *ny < carte.hauteur as isize &&
                                    !carte.est_obstacle(*nx, *ny)
                            })
                            .collect();

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

                        // Enregistrement de l'événement de déplacement
                        evenements.events.push(Evenement::RobotDeplace {
                            robot_id: entity.index() as u32,
                            from: (robot.x, robot.y),
                            to: (nouveau_x, nouveau_y),
                        });

                        robot.x = nouveau_x;
                        robot.y = nouveau_y;
                        transform.translation.x = nouveau_x as f32 * TAILLE_CASE - (carte.largeur as f32 * TAILLE_CASE) / 2.0;
                        transform.translation.y = nouveau_y as f32 * TAILLE_CASE - (carte.hauteur as f32 * TAILLE_CASE) / 2.0;

                        if nouveau_x >= 0 && nouveau_y >= 0 && nouveau_x < carte.largeur as isize && nouveau_y < carte.hauteur as isize {
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
                            // Calcul asynchrone du chemin via le worker pool
                            let current_pos = (robot.x, robot.y);
                            let target = cible;
                            let carte_clone = carte.clone(); // Clonage pour la tâche
                            let chemin_future = task_pool.spawn(async move {
                                calculer_chemin_bfs(&carte_clone, current_pos, target)
                            });
                            if let Some(chemin) = future::block_on(chemin_future) {
                                if chemin.len() > 1 {
                                    let (nx, ny) = chemin[1];
                                    evenements.events.push(Evenement::RobotDeplace {
                                        robot_id: entity.index() as u32,
                                        from: (robot.x, robot.y),
                                        to: (nx, ny),
                                    });
                                    robot.x = nx;
                                    robot.y = ny;
                                    transform.translation.x = nx as f32 * TAILLE_CASE - (carte.largeur as f32 * TAILLE_CASE) / 2.0;
                                    transform.translation.y = ny as f32 * TAILLE_CASE - (carte.hauteur as f32 * TAILLE_CASE) / 2.0;
                                }
                            } else {
                                println!("Explorateur bloqué/aucun chemin vers la station");
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
                                        evenements.events.push(Evenement::RessourceCollectee {
                                            robot_id: entity.index() as u32,
                                            resource: resource_type,
                                            position: (cx, cy),
                                        });
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
                                        if depot.stock_energie >= 3 {
                                            depot.stock_energie -= 3;
                                            println!("3 énergies accumulées ! Création d'un nouveau collecteur spécialisé en minerais.");
                                            creer_collecteur(&mut commandes, &station, ModuleRobot::Forage);
                                            evenements.events.push(Evenement::NouveauRobotCree {
                                                robot_role: RoleRobot::Collecteur,
                                                modules: vec![ModuleRobot::Forage],
                                            });
                                        }
                                    }
                                    TypePixel::Minerai => {
                                        depot.stock_minerai += 1;
                                        if depot.stock_minerai >= 3 {
                                            depot.stock_minerai -= 3;
                                            println!("3 minerais accumulés ! Création d'un nouveau collecteur spécialisé en énergie.");
                                            creer_collecteur(&mut commandes, &station, ModuleRobot::AnalyseChimique);
                                            evenements.events.push(Evenement::NouveauRobotCree {
                                                robot_role: RoleRobot::Collecteur,
                                                modules: vec![ModuleRobot::AnalyseChimique],
                                            });
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
                                println!("Collecteur bloqué : aucun chemin vers la station !");
                            }
                        }
                    },
                }
            },
        }
    }
}

/// Crée un nouveau collecteur à la station selon le module demandé.
pub fn creer_collecteur(
    commandes: &mut Commands,
    station: &PositionStation,
    module: ModuleRobot,
) {
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
