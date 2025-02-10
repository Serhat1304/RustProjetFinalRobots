use rand::Rng;

const LARGEUR: usize = 10;
const HAUTEUR: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Terrain {
    Vide,
    Obstacle,
    Ressource,
    Base,
}

impl Terrain {
    fn generer_aleatoire() -> Self {
        let mut rng = rand::thread_rng();
        match rng.gen_range(0..=100) {
            0..=60 => Terrain::Vide,
            61..=80 => Terrain::Obstacle,
            _ => Terrain::Ressource,
        }
    }

    fn symbole(&self) -> char {
        match self {
            Terrain::Vide => '.',
            Terrain::Obstacle => '#',
            Terrain::Ressource => 'R',
            Terrain::Base => 'B',
        }
    }
}

struct Carte {
    grille: [[Terrain; LARGEUR]; HAUTEUR],
}

impl Carte {
    fn nouvelle() -> Self {
        let mut grille = [[Terrain::Vide; LARGEUR]; HAUTEUR];

        let mut rng = rand::thread_rng();
        let base_x = rng.gen_range(0..LARGEUR);
        let base_y = rng.gen_range(0..HAUTEUR);
        grille[base_y][base_x] = Terrain::Base;

        for y in 0..HAUTEUR {
            for x in 0..LARGEUR {
                if grille[y][x] != Terrain::Base {
                    grille[y][x] = Terrain::generer_aleatoire();
                }
            }
        }

        Self { grille }
    }

    fn afficher(&self) {
        println!("Carte 2D :");
        for y in 0..HAUTEUR {
            for x in 0..LARGEUR {
                print!("{} ", self.grille[y][x].symbole());
            }
            println!();
        }
    }
}

fn main() {
    let carte = Carte::nouvelle();
    carte.afficher();
}