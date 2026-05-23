//! Mine — ennemi piège qui tombe verticalement.
//!
//! Flow :
//! 1. **falling** : la mine tombe en ligne droite, est `Invulnerable` (pas de
//!    dégâts pris) et `Harmless` (pas de dégâts au contact). Son `PlayerDetection`
//!    en cercle attend l'arrivée du joueur.
//! 2. Quand le joueur entre dans la zone → message `"player_detected"`, la
//!    choice transite vers `counting_down`. Le countdown est **engagé** : pas
//!    de transition retour.
//! 3. **counting_down** : la mine continue de tomber pendant `MINE_COUNTDOWN_DURATION`.
//!    À la fin du timer, `on_complete` pousse `"countdown_done"`.
//! 4. **exploding** : insère le marker `MineExplode`. Le système
//!    `mine_explode_system` détecte ce marker, spawn une `AreaOfEffect`
//!    centrée sur la position de la mine, et despawn la mine.

use std::time::Duration;

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::audio::{Sfx, SfxPlayer, spawn_sfx};
use crate::behavior::BehaviorBuilder;
use crate::behavior::behavior::BehaviorComponent;
use crate::behavior::choice_list::TransitionMessages;
use crate::enemy::anim_bank::Animation;
use crate::enemy::enemies::MINE;
use crate::enemy::enemy::Enemy;
use crate::enemy::enemy_builder::EnemyBuilder;
use crate::game_manager::difficulty::{Difficulty, SpawnPosition};
use crate::geometry::shape::Shape;
use crate::movement::despawn_off_screen::DespawnOffScreen;
use crate::movement::movements::Movements;
use crate::movement::translate::Translate;
use crate::physic::area_of_effect::spawn_aoe_animated;
use crate::physic::collider::{collider, layers};
use crate::physic::harmless::Harmless;
use crate::physic::health::Health;
use crate::physic::invulnerable::Invulnerable;
use crate::physic::player_detection::PlayerDetection;

const MINE_FALL_SPEED: f32 = 250.0;
const MINE_DETECTION_RADIUS: f32 = 225.0;
/// Rayon du collider AOE (px). Découplé du sprite : doit matcher le cercle
/// **visible** à l'intérieur du PNG (qui a une marge transparente). Garde
/// donc une valeur < `MINE_AOE_SPRITE_SIZE / 2`.
const MINE_AOE_RADIUS: f32 = 105.0;
const MINE_COUNTDOWN_DURATION: f32 = 1.5;
/// Durée de vie de l'AOE de la mine (s). Zone de denial qui persiste.
const MINE_AOE_LIFETIME: f32 = 2.5;
/// Taille rendue du sprite AOE (px). Indépendant du collider — correspond
/// aux dimensions natives du PNG (marge transparente incluse).
const MINE_AOE_SPRITE_SIZE: f32 = 270.0;
/// Période du clignotement rouge pendant le countdown (secondes par cycle).
const MINE_BLINK_PERIOD: f32 = 0.25;
/// Intervalle entre les 3 bips du countdown (secondes). Avec une durée de
/// countdown de 1.5s, 3 bips à intervalle 0.5 → bips à 0s, 0.5s, 1.0s,
/// explosion à 1.5s.
const MINE_BEEP_INTERVAL: f32 = 0.5;
const MINE_BEEP_COUNT: u8 = 3;
/// Durée par frame de l'animation idle de la mine (secondes).
const MINE_FRAME_DURATION: f32 = 0.15;

/// Marqueur sur l'entité Mine — pratique pour le debug / filter queries.
#[derive(Component)]
pub struct Mine;

/// Marker posé par la choice quand la mine doit exploser. Consommé par
/// `mine_explode_system` (spawn AOE + despawn mine). Le hook `on_insert`
/// joue `Sfx::MineExplode`.
#[derive(Component, Clone)]
#[component(on_insert = play_mine_explode)]
pub struct MineExplode;

fn play_mine_explode(mut world: DeferredWorld, _: HookContext) {
    spawn_sfx(&mut world, Sfx::MineExplode);
}

/// Composant inséré pendant la phase counting_down. Le système
/// `mine_countdown_audio` tick `elapsed` chaque frame et joue un bip à chaque
/// `MINE_BEEP_INTERVAL`, jusqu'à `MINE_BEEP_COUNT` bips au total.
#[derive(Component, Clone, Default)]
pub struct MineCountdownAudio {
    pub elapsed: f32,
    pub beeps_played: u8,
}

/// Modulation de la couleur du sprite vers le rouge en sinusoïde. Composant
/// inséré pendant le countdown via `ComponentContainer`, retiré quand la phase
/// se termine — le sprite reste donc à sa dernière couleur, mais la mine
/// explose dans la frame qui suit donc invisible.
#[derive(Component, Clone)]
pub struct BlinkRed {
    pub period: f32,
    pub elapsed: f32,
}

impl BlinkRed {
    pub fn new(period: f32) -> Self {
        Self {
            period,
            elapsed: 0.0,
        }
    }
}

pub struct MineBuilder {
    timer: Timer,
}
impl MineBuilder {
    pub fn new() -> Self {
        Self {
            timer: Timer::new(Duration::ZERO, TimerMode::Once),
        }
    }
}
impl EnemyBuilder for MineBuilder {
    fn get_timer(&mut self) -> &mut Timer {
        &mut self.timer
    }
    fn name(&self) -> &'static str {
        "mine"
    }
    fn preload_anim(&self) -> HashMap<&str, &str> {
        HashMap::from([
            ("mine", "images/mine"),
            ("mine_explosion", "images/mine/explosion"),
        ])
    }
    fn spawn(
        &self,
        mut commands: Commands,
        window: &Window,
        _difficulty: &ResMut<Difficulty>,
        spawn_pos: SpawnPosition,
        asset_server: &Res<AssetServer>,
    ) {
        let pos = spawn_pos.resolve(window, MINE.config.sprite_size / 2.0);

        let falling = BehaviorBuilder::from_component(
            Movements::new().with(Translate::new(Vec2::new(0.0, -1.0), MINE_FALL_SPEED)),
        );

        // Pendant le countdown la mine continue à tomber, clignote rouge, et
        // un système joue le son d'amorçage (via le marker MineCountingDown).
        // `on_complete` pousse "countdown_done" quand le timer expire → choice
        // transite vers index 2.
        let counting_down = BehaviorBuilder::first(
            Duration::from_secs_f32(MINE_COUNTDOWN_DURATION),
            BehaviorBuilder::multiple()
                .with(BehaviorBuilder::from_component(
                    Movements::new().with(Translate::new(Vec2::new(0.0, -1.0), MINE_FALL_SPEED)),
                ))
                .with(BehaviorBuilder::from_component(BlinkRed::new(
                    MINE_BLINK_PERIOD,
                )))
                .with(BehaviorBuilder::from_component(
                    MineCountdownAudio::default(),
                )),
        )
        .on_complete("countdown_done");

        let exploding = BehaviorBuilder::from_component(MineExplode);

        // Pas de transition retour vers 0 — une fois le countdown engagé, la
        // mine va forcément exploser.
        let behavior = BehaviorBuilder::choice()
            .with(falling) // 0
            .with(counting_down) // 1
            .with(exploding) // 2
            .add_transition(0, 1, "player_detected")
            .add_transition(1, 2, "countdown_done");

        commands.spawn((
            Sprite {
                image: asset_server.load("images/mine/frame000.png"),
                custom_size: Some(Vec2::splat(MINE.config.sprite_size)),
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y, 0.5),
            TransitionMessages::new(),
            Enemy::new(MINE),
            Health::new(MINE.total_hp),
            Animation::new("mine", Duration::from_secs_f32(MINE_FRAME_DURATION)),
            DespawnOffScreen,
            Invulnerable,
            Harmless,
            Mine,
            PlayerDetection {
                shape: Shape::Circle(MINE_DETECTION_RADIUS),
                on_enter: Some("player_detected"),
                on_exit: None,
                inside: false,
                cooldown_duration: 0.0,
                cooldown_remaining: 0.0,
            },
            BehaviorComponent::new(behavior),
            // Mine porte ENEMY layer. Sa nature `Invulnerable + Harmless`
            // est gérée par les reactive systems (qui skip Harmless pour
            // damage joueur et Invulnerable pour damage par projectile).
            collider(
                Shape::Circle(MINE.config.radius),
                layers::ENEMY,
                layers::PLAYER | layers::PLAYER_PROJECTILE,
            ),
        ));
    }
}

/// Détecte le marker `MineExplode` posé par la choice : spawn l'AOE à la
/// position courante de la mine, joue le son d'explosion sur une entité
/// indépendante, puis despawn la mine.
pub fn mine_explode_system(
    mut commands: Commands,
    anim_bank: Res<crate::enemy::anim_bank::AnimBank>,
    query: Query<(Entity, &Transform), With<MineExplode>>,
) {
    for (entity, transform) in &query {
        spawn_aoe_animated(
            &mut commands,
            &anim_bank,
            transform.translation,
            Shape::Circle(MINE_AOE_RADIUS),
            MINE_AOE_LIFETIME,
            "mine_explosion",
            MINE_AOE_SPRITE_SIZE,
        );
        commands.trigger(crate::fx::screen_shake::ScreenShakeEvent::MINE);
        // Son `MineExplode` joué par le hook `on_insert` sur `MineExplode`.
        // DespawnSelf au lieu de try_despawn direct : évite la race avec les
        // commandes du behavior tree (cf. collision.rs pour les détails).
        if let Ok(mut e) = commands.get_entity(entity) {
            e.try_insert(crate::enemy::death::DespawnSelf);
        }
    }
}

/// Joue 3 bips successifs pendant la phase counting_down de chaque mine.
/// Tick `elapsed` chaque frame et spawn une entité audio par bip à atteindre.
/// `while` (et non `if`) pour rattraper plusieurs intervalles si un frame
/// très lourd skip plus de `MINE_BEEP_INTERVAL` d'un coup.
pub fn mine_countdown_audio(
    mut sfx: SfxPlayer,
    time: Res<Time>,
    mut q: Query<&mut MineCountdownAudio>,
) {
    let dt = time.delta_secs();
    for mut audio in &mut q {
        audio.elapsed += dt;
        while audio.beeps_played < MINE_BEEP_COUNT
            && (audio.beeps_played as f32) * MINE_BEEP_INTERVAL <= audio.elapsed
        {
            sfx.play(Sfx::MineBeep);
            audio.beeps_played += 1;
        }
    }
}

/// Module la couleur du sprite des entités avec `BlinkRed` : sin wave entre
/// blanc et rouge sur `period` secondes par cycle.
pub fn blink_red_system(time: Res<Time>, mut query: Query<(&mut BlinkRed, &mut Sprite)>) {
    let dt = time.delta_secs();
    for (mut blink, mut sprite) in &mut query {
        blink.elapsed += dt;
        // sin oscille en [-1, 1] → t en [0, 1] (0 = blanc, 1 = rouge saturé)
        let t = (blink.elapsed * std::f32::consts::TAU / blink.period).sin() * 0.5 + 0.5;
        let inv = 1.0 - t;
        sprite.color = Color::srgba(1.0, inv, inv, 1.0);
    }
}
