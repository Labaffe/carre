//! Simple UFO — petit ennemi sans état, suit un **chemin Bézier paramétré**
//! et despawn à la sortie de l'écran.
//!
//! ## Système de waves
//!
//! Conçu pour le pattern "queue leu leu" via [`SimpleUfoWaveSpawner`] :
//! une entité contrôleur génère **un chemin aléatoire à sa création**, puis
//! spawn N UFOs à intervalle régulier le long de CE chemin. Comme tous les
//! UFOs de la même wave partagent strictement les mêmes 4 paramètres
//! (start, mid, end, duration), ils se retrouvent à des positions
//! différentes le long de la courbe au même instant → file ondulante.
//!
//! Pour chaîner plusieurs waves (chacune avec son propre chemin random),
//! le pattern attendu est de chaîner plusieurs [`Action::SpawnSimpleUfoWave`]
//! (cf. [`crate::level::level`]) à différents temps dans la timeline d'un
//! niveau. Chaque action spawn un `SimpleUfoWaveSpawner` distinct avec sa
//! propre paire de paramètres random → trajectoires variées.
//!
//! ## Architecture
//!
//! - [`simple_ufo_bundle`] : bundle complet d'un UFO (Sprite, Health,
//!   collider, Movements::Bezier). Réutilisable depuis n'importe quel
//!   spawn site (wave spawner, builder standalone, autre).
//! - [`SimpleUfoBuilder`] : interface `EnemyBuilder` pour spawn manuel d'1
//!   UFO via `Action::SpawnEnemy("simple_ufo", ...)`. Chemin déterministe
//!   (cf. [`default_path`]) pour les tests unitaires.
//! - [`SimpleUfoWaveSpawner`] + [`simple_ufo_wave_spawn_system`] : système
//!   de spawn en queue. À utiliser via `Action::SpawnSimpleUfoWave`.
//! - [`random_path`] : génère une trajectoire random respectant un set de
//!   contraintes (entrée d'un côté, sortie de l'autre).

use std::time::Duration;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::behavior::BehaviorBuilder;
use crate::behavior::behavior::BehaviorComponent;
use crate::behavior::choice_list::TransitionMessages;
use crate::enemy::anim_bank::Animation;
use crate::enemy::death::DespawnSelf;
use crate::enemy::enemies::SIMPLE_UFO;
use crate::enemy::enemy::Enemy;
use crate::enemy::enemy_builder::EnemyBuilder;
use crate::game_manager::difficulty::{Difficulty, SpawnPosition};
use crate::geometry::shape::Shape;
use crate::movement::bezier::Bezier;
use crate::movement::despawn_off_screen::DespawnOffScreen;
use crate::movement::movements::Movements;
use crate::physic::collider::{collider, layers};
use crate::physic::health::Health;
use crate::sprite_orient::RotateToMovement;

/// Marge (px) au-delà du bord d'écran pour les points d'entrée/sortie —
/// garantit qu'un UFO commence et finit hors champ (DespawnOffScreen
/// déclenche à la sortie).
const PATH_OFFSCREEN_MARGIN: f32 = 100.0;
/// Durée par frame de l'animation idle en boucle.
const SIMPLE_UFO_ANIM_FRAME_DURATION: f32 = 0.08;

// ─── Bundle UFO + spawn helper ──────────────────────────────────────

/// Bundle complet d'un Simple UFO le long d'un chemin Bézier paramétré.
/// La position initiale du Transform = `start` (utile pour le rendu au
/// premier frame avant que `movement_driver` ne s'exécute).
fn simple_ufo_bundle(
    asset_server: &Res<AssetServer>,
    start: Vec2,
    mid: Vec2,
    end: Vec2,
    duration: f32,
) -> impl Bundle {
    // BT minimal : `alive` injecte animation + bézier ; sur HP=0,
    // `detect_death` pousse "die" → transition vers `dying` qui insère
    // `DespawnSelf`. **Indispensable** : `detect_death` exige
    // `TransitionMessages` dans son query — sans BT le UFO ne meurt pas.
    let alive = BehaviorBuilder::multiple()
        .with(BehaviorBuilder::from_component(Animation::new(
            "simple_ufo",
            Duration::from_secs_f32(SIMPLE_UFO_ANIM_FRAME_DURATION),
        )))
        .with(BehaviorBuilder::from_component(Movements::new().with(
            Bezier::passing_through(start, mid, end, Duration::from_secs_f32(duration)),
        )));
    let dying = BehaviorBuilder::from_component(DespawnSelf);
    let behavior = BehaviorBuilder::choice()
        .with(alive)
        .with(dying)
        .add_transition(0, 1, "die");

    (
        Sprite {
            image: asset_server.load("images/simple_ufo/frame000.png"),
            custom_size: Some(Vec2::splat(SIMPLE_UFO.config.sprite_size)),
            ..default()
        },
        Transform::from_xyz(start.x, start.y, 0.5),
        Enemy::new(SIMPLE_UFO),
        Health::new(SIMPLE_UFO.total_hp),
        DespawnOffScreen,
        // Rotation continue vers la tangente de la trajectoire Bézier.
        RotateToMovement::facing_up(),
        collider(
            Shape::Circle(SIMPLE_UFO.config.radius),
            layers::ENEMY,
            layers::PLAYER | layers::PLAYER_PROJECTILE,
        ),
        TransitionMessages::new(),
        BehaviorComponent::new(behavior),
    )
}

// ─── Wave spawner ──────────────────────────────────────────────────

/// Contrôleur d'une wave : pré-calcule un chemin Bézier au 1er tick (lazy,
/// car le Window n'est pas accessible au moment où l'action level fire le
/// spawn) puis spawn N UFOs à intervalle régulier le long de ce chemin.
/// Auto-despawn quand tous les UFOs ont été émis.
#[derive(Component)]
#[require(crate::GameplayEntity)]
pub struct SimpleUfoWaveSpawner {
    /// Path Bézier, rempli au 1er tick (`initialized = true` après).
    pub start: Vec2,
    pub mid: Vec2,
    pub end: Vec2,
    pub duration: f32,
    /// Faux jusqu'au 1er tick → `start/mid/end/duration` non valides avant.
    pub initialized: bool,
    pub remaining: usize,
    pub timer: Timer,
}

impl SimpleUfoWaveSpawner {
    /// Crée un spawner. Le chemin random sera calculé au 1er tick (besoin
    /// d'accès à la fenêtre).
    pub fn new(count: usize, interval: f32) -> Self {
        Self {
            start: Vec2::ZERO,
            mid: Vec2::ZERO,
            end: Vec2::ZERO,
            duration: 0.0,
            initialized: false,
            remaining: count,
            timer: Timer::from_seconds(interval, TimerMode::Repeating),
        }
    }
}

/// Tick chaque `SimpleUfoWaveSpawner` ; au 1er tick génère le chemin
/// random et spawn le 1er UFO immédiatement. Ensuite, à chaque
/// `just_finished` du timer (et tant que `remaining > 0`), spawn un UFO
/// supplémentaire le long du chemin pré-calculé. Quand `remaining`
/// atteint 0, le spawner se despawn lui-même.
pub fn simple_ufo_wave_spawn_system(
    mut commands: Commands,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    window: Single<&Window>,
    mut wave_q: Query<(Entity, &mut SimpleUfoWaveSpawner)>,
) {
    for (wave_e, mut wave) in &mut wave_q {
        // Lazy init du path random + spawn du 1er UFO immédiatement.
        if !wave.initialized {
            let (start, mid, end, duration) = random_path(&window);
            wave.start = start;
            wave.mid = mid;
            wave.end = end;
            wave.duration = duration;
            wave.initialized = true;
            if wave.remaining > 0 {
                commands.spawn(simple_ufo_bundle(
                    &asset_server,
                    start,
                    mid,
                    end,
                    duration,
                ));
                wave.remaining -= 1;
            }
        }

        wave.timer.tick(time.delta());
        if wave.timer.just_finished() && wave.remaining > 0 {
            commands.spawn(simple_ufo_bundle(
                &asset_server,
                wave.start,
                wave.mid,
                wave.end,
                wave.duration,
            ));
            wave.remaining -= 1;
        }
        if wave.remaining == 0 {
            if let Ok(mut e) = commands.get_entity(wave_e) {
                e.try_despawn();
            }
        }
    }
}

// ─── Génération de chemins ──────────────────────────────────────────

/// Génère un chemin Bézier random qui traverse l'écran d'un bord à
/// l'autre. Retourne `(start, mid, end, duration_s)`.
///
/// Contraintes :
/// - Entrée par le bord gauche OU droit (random), hors écran.
/// - Sortie par le bord opposé, hors écran.
/// - Hauteur d'entrée/sortie random dans la moitié haute de l'écran.
/// - Apex (point milieu) random dans la zone centrale.
/// - Durée random entre 3 et 5s (plus c'est court, plus l'UFO est rapide).
pub fn random_path(window: &Window) -> (Vec2, Vec2, Vec2, f32) {
    let half_w = window.width() / 2.0;
    let half_h = window.height() / 2.0;
    let m = PATH_OFFSCREEN_MARGIN;

    // Entrée à gauche (signe = -1) ou à droite (+1). Sortie opposée.
    let enter_left = fastrand::bool();
    let entry_sign = if enter_left { -1.0 } else { 1.0 };

    // Hauteur d'entrée et sortie : random dans [-0.3, +0.7] × half_h
    // (privilégie le haut de l'écran sans interdire le bas modéré).
    let entry_y = (fastrand::f32() * 1.0 - 0.3) * half_h;
    let exit_y = (fastrand::f32() * 1.0 - 0.3) * half_h;

    // Apex : random dans une bande horizontale centrale, hauteur libre.
    let mid_x = (fastrand::f32() * 0.8 - 0.4) * half_w;
    let mid_y = (fastrand::f32() * 1.2 - 0.6) * half_h;

    let start = Vec2::new(entry_sign * (half_w + m), entry_y);
    let end = Vec2::new(-entry_sign * (half_w + m), exit_y);
    let mid = Vec2::new(mid_x, mid_y);
    let duration = 3.0 + fastrand::f32() * 2.0;

    (start, mid, end, duration)
}

/// Chemin déterministe utilisé par le `SimpleUfoBuilder` standalone
/// (spawn unique via `Action::SpawnEnemy`). Forme en V plat.
fn default_path(window: &Window) -> (Vec2, Vec2, Vec2, f32) {
    let half_w = window.width() / 2.0;
    let half_h = window.height() / 2.0;
    let m = PATH_OFFSCREEN_MARGIN;
    (
        Vec2::new(-half_w - m, half_h * 0.4),
        Vec2::new(0.0, -half_h * 0.4),
        Vec2::new(half_w + m, half_h * 0.4),
        4.0,
    )
}

// ─── EnemyBuilder ──────────────────────────────────────────────────

pub struct SimpleUfoBuilder {
    timer: Timer,
}

impl SimpleUfoBuilder {
    pub fn new() -> Self {
        Self {
            timer: Timer::new(Duration::ZERO, TimerMode::Once),
        }
    }
}

impl EnemyBuilder for SimpleUfoBuilder {
    fn get_timer(&mut self) -> &mut Timer {
        &mut self.timer
    }
    fn name(&self) -> &'static str {
        "simple_ufo"
    }
    fn preload_anim(&self) -> HashMap<&str, &str> {
        HashMap::from([("simple_ufo", "images/simple_ufo")])
    }
    fn spawn(
        &self,
        mut commands: Commands,
        window: &Window,
        _difficulty: &ResMut<Difficulty>,
        _spawn_pos: SpawnPosition,
        asset_server: &Res<AssetServer>,
    ) {
        // Spawn unique → chemin déterministe (sinon pas reproductible
        // pour les tests). Pour des trajectoires random en queue, passer
        // par `Action::SpawnSimpleUfoWave`.
        let (start, mid, end, duration) = default_path(window);
        commands.spawn(simple_ufo_bundle(asset_server, start, mid, end, duration));
    }
}
