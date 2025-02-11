use rand::prelude::*;
use crate::common::{LARGEUR_CARTE, HAUTEUR_CARTE, TypePixel};

pub fn placer_station(carte: &mut Vec<Vec<TypePixel>>, generateur_aleatoire: &mut StdRng) -> (usize, usize) {
    loop {
        let x = generateur_aleatoire.gen_range(0..LARGEUR_CARTE);
        let y = generateur_aleatoire.gen_range(0..HAUTEUR_CARTE);

        if carte[y][x] == TypePixel::Vide {
            carte[y][x] = TypePixel::Station;
            return (x, y);
        }
    }
}

pub fn initialiser_station(mut commandes: bevy::prelude::Commands) {
    println!("Initialisation de la station.");
}