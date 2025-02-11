use rand::Rng;
use std::env;

pub fn obtenir_seed_depuis_arguments() -> Option<u64> {
    let arguments: Vec<String> = env::args().collect();
    if arguments.len() > 1 {
        arguments[1].parse::<u64>().ok()
    } else {
        None
    }
}

pub fn generer_seed_aleatoire() -> u64 {
    rand::thread_rng().gen::<u64>()
}