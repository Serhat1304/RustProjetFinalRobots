use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use rand::{prelude::*, SeedableRng};
use crate::obstacles::limiter_taille_obstacles;
use crate::station::placer_station;
use crate::common::{LARGEUR_CARTE, HAUTEUR_CARTE, TAILLE_CASE, TypePixel};

#[derive(Component)]
struct Pixel {
    type_pixel: TypePixel,
}

pub fn generer_map(mut commandes: Commands, seed_carte: Res<crate::SeedCarte>) {
    let bruit_perlin = Perlin::new(seed_carte.seed as u32);
    let mut rng = StdRng::seed_from_u64(seed_carte.seed);
    let mut carte = vec![vec![TypePixel::Vide; LARGEUR_CARTE]; HAUTEUR_CARTE];

    for y in 0..HAUTEUR_CARTE {
        for x in 0..LARGEUR_CARTE {
            let valeur_bruit = bruit_perlin.get([x as f64 * 0.1, y as f64 * 0.1]);
            if valeur_bruit > 0.5 {
                carte[y][x] = TypePixel::Obstacle;
            }
        }
    }

    limiter_taille_obstacles(&mut carte);

    for y in 0..HAUTEUR_CARTE {
        for x in 0..LARGEUR_CARTE {
            if carte[y][x] == TypePixel::Vide {
                carte[y][x] = match rng.gen_range(0..100) {
                    0..=5 => TypePixel::Energie,
                    6..=10 => TypePixel::Minerai,
                    11..=14 => TypePixel::SiteScientifique,
                    _ => TypePixel::Vide,
                };
            }
        }
    }

    let (pos_x, pos_y) = placer_station(&mut carte, &mut rng);
    println!("Station placée en ({}, {})", pos_x, pos_y);

    for y in 0..HAUTEUR_CARTE {
        for x in 0..LARGEUR_CARTE {
            let couleur = match carte[y][x] {
                TypePixel::Obstacle => Color::rgb(0.2, 0.2, 0.2),
                TypePixel::Energie => Color::rgb(1.0, 1.0, 0.0),
                TypePixel::Minerai => Color::rgb(0.5, 0.3, 0.1),
                TypePixel::SiteScientifique => Color::rgb(0.0, 0.8, 0.8),
                TypePixel::Station => Color::rgb(1.0, 0.0, 0.0),
                _ => Color::rgb(0.8, 0.8, 0.8),
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
            .insert(Pixel { type_pixel: carte[y][x] });
        }
    }
}