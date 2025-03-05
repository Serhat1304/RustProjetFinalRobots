# RustProjetFinalRobots

## Simulation de Robots sur Carte 2D en Rust

Ce projet est une simulation en 2D d'une carte générée aléatoirement sur laquelle évoluent des robots. La carte contient une station, des obstacles et trois types de ressources (énergie, minerais et sites scientifiques). Deux types de robots sont créés : pour explorer la carte et collecter les ressources.


## Introduction

Ce projet Rust utilise le moteur de jeu [Bevy](https://bevyengine.org/) pour créer une simulation interactive dans laquelle :
- Une carte 2D est générée de façon procédurale.
- Des obstacles sont placés selon un bruit de Perlin.
- Des ressources et sites scientifiques sont générés aléatoirement dans des pourcentages définis.
- Des robots, de rôles différents (explorateurs et collecteurs), se déplacent sur la carte pour découvrir et collecter ces ressources.

## Fonctionnalités

- **Génération de carte** : Création d’une grille 2D avec obstacles, ressources (énergie et minerais) et sites scientifiques.
- **Station de base** : Placement d’une station sur la carte servant de point de dépôt.
- **Robots autonomes** : Deux types de robots avec des comportements d’exploration et de collecte.
- **Événement et log** : Chaque déplacement et action est enregistré et affiché dans la console.
- **Tests unitaires** : Plusieurs tests sont implémentés pour valider la génération de la carte, le placement de la station, et le comportement des robots.

## Architecture du Projet

Le projet est organisé en plusieurs modules pour séparer les responsabilités :

- **src/main.rs**  
  Point d'entrée de l’application qui initialise l’environnement Bevy et insère les ressources nécessaires (seed, station, événements).

- **src/carte.rs**  
  Contient la logique de génération de la carte, le placement des obstacles, des ressources, et de la station.  
  *Points de configuration importants* :  
  - `LARGEUR_CARTE`, `HAUTEUR_CARTE`  
  - `SEUIL_OBSTACLE` pour le bruit de Perlin  
  - La répartition des ressources est définie dans un `match` sur un nombre aléatoire (voir section [Modification des Pourcentages](#modification-des-pourcentages-de-génération-des-éléments-de-la-carte)).

- **src/robot.rs**  
  Gère la création et le comportement des robots. Les robots sont divisés en deux rôles (explorateurs et collecteurs) et comportent des modules spécifiques qui définissent leurs capacités.

- **src/systemes.rs**  
  Définit les systèmes Bevy tels que l’initialisation de la caméra, la configuration de la minuterie pour la vitesse des robots, et la synchronisation des sprites avec les données de la carte.

- **src/utils.rs**  
  Fournit des utilitaires comme le calcul de chemin (BFS), la gestion des événements, la récupération ou génération d’un seed et l’enregistrement des découvertes.

## Installation et Exécution

### Prérequis
- [Rust](https://www.rust-lang.org/tools/install) (version stable recommandée)
- [Cargo](https://doc.rust-lang.org/cargo/) (installé avec Rust)

### Compilation et exécution
Pour compiler et lancer le projet, exécutez :


```bash
cargo run
```

Lors du lancement, le programme récupère un seed depuis la ligne de commande s’il est fourni, ou en génère un aléatoirement. Le seed utilisé est affiché dans la console.

## Configuration et Paramétrage

### Lancement avec un Seed Spécifique
Pour reproduire une carte spécifique, lancez le programme en passant le seed en argument :

```bash
cargo run -- 123456789
```
Ici, 123456789 est le seed qui sera utilisé pour la génération de la carte.

### Modification de la Vitesse des Robots

La vitesse de déplacement des robots est contrôlée via une minuterie dans le module src/systemes.rs.
Recherchez cette ligne :

```rust
Timer::from_seconds(0.3, TimerMode::Repeating)
```

### Modification du Nombre de Robots

Le nombre de robots est défini dans le module src/robot.rs dans la fonction creer_robots.
Les variables suivantes contrôlent le nombre de chaque type de robot par défaut :
```rust
let nb_explorateurs = 3;
let nb_collecteurs_analyse = 1;
let nb_collecteurs_forage = 1;
```

### Modification des Pourcentages de Génération des Éléments de la Carte
La répartition des éléments (énergie, minerais, site scientifique) sur la carte est réalisée dans la fonction generer_carte du module src/carte.rs.

Voici le bloc de code concerné :

```rust
carte[y][x] = match generateur_aleatoire.gen_range(0..100) {
    0..=5   => TypePixel::Energie,
    6..=10  => TypePixel::Minerai,
    11..=14 => TypePixel::SiteScientifique,
    _       => TypePixel::Vide,
};
```
- Énergie : La plage 0..=5 signifie environ 6% de chances.
- Minerais : La plage 6..=10 représente environ 5% de chances.
- Sites Scientifiques : La plage 11..=14 correspond à environ 4% de chances.
