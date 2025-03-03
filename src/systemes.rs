use bevy::prelude::*;
use std::collections::HashSet;

/// Ressource pour contrôler la fréquence de déplacement des robots.
/// Modifier le temps ici permet de changer la vitesse globale de déplacement.
#[derive(Resource)]
pub struct MinuterieRobot {
    pub timer: Timer,
}

/// Ressource indiquant si les robots ont été créés (pour éviter de recréer des robots à chaque frame).
#[derive(Resource)]
pub struct RobotsCrees(pub bool);

/// Système d'initialisation de la caméra 2D.
pub fn initialiser_camera(mut commandes: Commands) {
    commandes.spawn(Camera2dBundle::default());
}

/// Système d'initialisation de la minuterie.
/// Modifier le temps dans `Timer::from_seconds` permet de régler la vitesse des robots.
pub fn configurer_minuterie_robot(mut commandes: Commands) {
    commandes.insert_resource(MinuterieRobot {
        timer: Timer::from_seconds(0.7, TimerMode::Repeating),
    });
}

/// Système qui initialise la ressource indiquant que les robots n'ont pas encore été créés.
pub fn initialiser_robots_crees(mut commandes: Commands) {
    commandes.insert_resource(RobotsCrees(false));
}

/// Système de synchronisation des pixels de la carte avec leur sprite correspondant.
/// Il met à jour la couleur du sprite en fonction du type de pixel.
/// --- Code couleurs des pixels ---
/// Obstacle          -> gris foncé (0.2, 0.2, 0.2)
/// Energie           -> jaune (1.0, 1.0, 0.0)
/// Minerai           -> marron (0.5, 0.3, 0.1)
/// SiteScientifique  -> cyan (0.0, 0.8, 0.8)
/// Station           -> rouge (1.0, 0.0, 0.0)
/// Vide              -> gris clair (0.8, 0.8, 0.8)
pub fn synchroniser_pixels_carte(
    carte: Res<crate::carte::Carte>,
    mut requete: Query<(&mut crate::carte::Pixel, &mut Sprite, &Transform)>,
) {
    for (mut pixel, mut sprite, transform) in requete.iter_mut() {
        let tile_x = ((transform.translation.x + (carte.largeur as f32 * crate::carte::TAILLE_CASE) / 2.0)
            / crate::carte::TAILLE_CASE)
            .round() as usize;
        let tile_y = ((transform.translation.y + (carte.hauteur as f32 * crate::carte::TAILLE_CASE) / 2.0)
            / crate::carte::TAILLE_CASE)
            .round() as usize;
        if tile_x < carte.largeur && tile_y < carte.hauteur {
            let nouveau_type = carte.donnees[tile_y][tile_x];
            if pixel.type_pixel != nouveau_type {
                pixel.type_pixel = nouveau_type;
                sprite.color = match nouveau_type {
                    crate::carte::TypePixel::Obstacle => Color::rgb(0.2, 0.2, 0.2),
                    crate::carte::TypePixel::Energie => Color::rgb(1.0, 1.0, 0.0),
                    crate::carte::TypePixel::Minerai => Color::rgb(0.5, 0.3, 0.1),
                    crate::carte::TypePixel::SiteScientifique => Color::rgb(0.0, 0.8, 0.8),
                    crate::carte::TypePixel::Station => Color::rgb(1.0, 0.0, 0.0),
                    crate::carte::TypePixel::Vide => Color::rgb(0.8, 0.8, 0.8),
                };
            }
        }
    }
}