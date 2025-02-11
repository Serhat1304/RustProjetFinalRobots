pub const LARGEUR_CARTE: usize = 50;
pub const HAUTEUR_CARTE: usize = 30;
pub const TAILLE_CASE: f32 = 20.0;
pub const MAX_TAILLE_OBSTACLE: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TypePixel {
    Vide,
    Obstacle,
    Energie,
    Minerai,
    SiteScientifique,
    Station,
}