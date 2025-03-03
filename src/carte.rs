use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use rand::{prelude::*, SeedableRng};

/// Largeur de la carte (utilisée pour la génération principale).
pub const LARGEUR_CARTE: usize = 50;
/// Hauteur de la carte (utilisée pour la génération principale).
pub const HAUTEUR_CARTE: usize = 30;
/// Taille d'une case en pixels.
pub const TAILLE_CASE: f32 = 20.0;
/// Seuil du bruit de Perlin pour déterminer la présence d'obstacles.
pub const SEUIL_OBSTACLE: f64 = 0.5;
/// Taille maximale d'un groupe d'obstacles connectés.
pub const TAILLE_MAX_OBSTACLE: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TypePixel {
    Vide,
    Obstacle,
    Energie,
    Minerai,
    SiteScientifique,
    Station,
}

/// Composant utilisé pour l'affichage de chaque case (pixel) sur la carte.
#[derive(Component)]
pub struct Pixel {
    pub type_pixel: TypePixel,
}

/// Ressource contenant le seed utilisé pour la génération de la carte.
#[derive(Resource)]
pub struct SeedCarte {
    pub seed: u64,
}

/// Ressource représentant la carte avec ses données, largeur et hauteur.
/// Le derive Clone permet de cloner la carte pour des calculs asynchrones ou tests.
#[derive(Resource, Clone)]
pub struct Carte {
    pub donnees: Vec<Vec<TypePixel>>,
    pub largeur: usize,
    pub hauteur: usize,
}

impl Carte {
    /// Vérifie si la position (x, y) est hors limites ou correspond à un obstacle.
    pub fn est_obstacle(&self, x: isize, y: isize) -> bool {
        if x < 0 || y < 0 || x >= self.largeur as isize || y >= self.hauteur as isize {
            return true;
        }
        self.donnees[y as usize][x as usize] == TypePixel::Obstacle
    }
}

/// Ressource contenant la position de la station sur la carte.
#[derive(Resource)]
pub struct PositionStation {
    pub x: usize,
    pub y: usize,
}

/// Ressource utilisée par la station pour stocker les découvertes et les ressources collectées.
#[derive(Resource, Debug)]
pub struct DepotStation {
    pub decouvertes: Vec<Decouverte>,
    pub stock_energie: u32,
    pub stock_minerai: u32,
}

/// Structure représentant une découverte d'une ressource sur la carte.
#[derive(Debug, Clone)]
pub struct Decouverte {
    pub resource: TypePixel,
    pub x: isize,
    pub y: isize,
}

/// Génère la carte en créant une grille avec des obstacles et des ressources, puis place la station.
/// Les sprites sont créés avec le code couleur suivant :
/// - Obstacle          -> gris foncé (0.2, 0.2, 0.2)
/// - Energie           -> jaune (1.0, 1.0, 0.0)
/// - Minerai           -> marron (0.5, 0.3, 0.1)
/// - SiteScientifique  -> cyan (0.0, 0.8, 0.8)
/// - Station           -> rouge (1.0, 0.0, 0.0)
/// - Vide              -> gris clair (0.8, 0.8, 0.8)
pub fn generer_carte(mut commandes: Commands, seed_carte: Res<SeedCarte>) {
    println!("Seed Actuel: {}", seed_carte.seed);

    let bruit_perlin = Perlin::new(seed_carte.seed as u32);
    let mut generateur_aleatoire = rand::rngs::StdRng::seed_from_u64(seed_carte.seed);

    // Création de la carte en utilisant les constantes LARGEUR_CARTE et HAUTEUR_CARTE
    let mut carte = vec![vec![TypePixel::Vide; LARGEUR_CARTE]; HAUTEUR_CARTE];

    // Placement des obstacles selon le bruit de Perlin
    for y in 0..HAUTEUR_CARTE {
        for x in 0..LARGEUR_CARTE {
            let valeur_bruit = bruit_perlin.get([x as f64 * 0.1, y as f64 * 0.1]);
            if valeur_bruit > SEUIL_OBSTACLE {
                carte[y][x] = TypePixel::Obstacle;
            }
        }
    }

    limiter_taille_obstacles(&mut carte);

    // Remplissage aléatoire des cases vides avec des ressources ou des sites scientifiques
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

    // Placement de la station sur une case vide
    let (pos_x, pos_y) = placer_station(&mut carte, &mut generateur_aleatoire);
    println!("Station placée en ({}, {})", pos_x, pos_y);

    commandes.insert_resource(Carte {
        donnees: carte.clone(),
        largeur: LARGEUR_CARTE,
        hauteur: HAUTEUR_CARTE,
    });
    commandes.insert_resource(PositionStation { x: pos_x, y: pos_y });

    // Création des sprites pour chaque case en appliquant le code couleur
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

            commandes.spawn(SpriteBundle {
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

/// Place la station sur une case vide de la grille et retourne ses coordonnées.
/// Cette fonction utilise les dimensions réelles de la grille passée en paramètre.
fn placer_station(
    carte: &mut Vec<Vec<TypePixel>>,
    generateur_aleatoire: &mut rand::rngs::StdRng,
) -> (usize, usize) {
    let hauteur = carte.len();
    let largeur = if hauteur > 0 { carte[0].len() } else { 0 };
    loop {
        let x = generateur_aleatoire.gen_range(0..largeur);
        let y = generateur_aleatoire.gen_range(0..hauteur);
        if carte[y][x] == TypePixel::Vide {
            carte[y][x] = TypePixel::Station;
            return (x, y);
        }
    }
}

/// Limite la taille des obstacles en parcourant la grille en fonction de ses dimensions réelles.
/// Si un groupe d'obstacles dépasse TAILLE_MAX_OBSTACLE, certaines cases sont transformées en Vide.
fn limiter_taille_obstacles(carte: &mut Vec<Vec<TypePixel>>) {
    let hauteur = carte.len();
    let largeur = if hauteur > 0 { carte[0].len() } else { 0 };
    let directions = [(0, 1), (1, 0), (0, -1), (-1, 0)];
    for y in 0..hauteur {
        for x in 0..largeur {
            if carte[y][x] == TypePixel::Obstacle {
                let mut taille_obstacle = 1;
                for (dx, dy) in directions.iter() {
                    let mut nx = x as isize + dx;
                    let mut ny = y as isize + dy;
                    while nx >= 0 && (nx as usize) < largeur &&
                        ny >= 0 && (ny as usize) < hauteur &&
                        carte[ny as usize][nx as usize] == TypePixel::Obstacle {
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

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn test_est_obstacle() {
        // Création d'une carte 2x2 simple
        let donnees = vec![
            vec![TypePixel::Vide, TypePixel::Obstacle],
            vec![TypePixel::Vide, TypePixel::Vide],
        ];
        let carte = Carte {
            donnees,
            largeur: 2,
            hauteur: 2,
        };
        assert!(carte.est_obstacle(1, 0));
        assert!(!carte.est_obstacle(0, 0));
        // Test des bornes
        assert!(carte.est_obstacle(-1, 0));
        assert!(carte.est_obstacle(0, 2));
    }

    #[test]
    fn test_placer_station() {
        // Création d'une carte 3x3 remplie de Vide
        let mut donnees = vec![vec![TypePixel::Vide; 3]; 3];
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let (x, y) = placer_station(&mut donnees, &mut rng);
        // La case choisie doit désormais être Station
        assert_eq!(donnees[y][x], TypePixel::Station);
        // Les coordonnées doivent être dans les limites de la grille
        assert!(x < 3 && y < 3);
    }

    #[test]
    fn test_limiter_taille_obstacles() {
        // Création d'une carte 1x7 avec des obstacles continus
        let mut donnees = vec![vec![TypePixel::Obstacle; 7]];
        // Avant limitation, la ligne contient 7 obstacles
        let count_before = donnees[0].iter().filter(|&&p| p == TypePixel::Obstacle).count();
        assert_eq!(count_before, 7);
        limiter_taille_obstacles(&mut donnees);
        // Après limitation, on s'attend à ce qu'au moins un obstacle soit transformé en Vide
        let count_after = donnees[0].iter().filter(|&&p| p == TypePixel::Obstacle).count();
        assert!(count_after < 7);
    }
}
