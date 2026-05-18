//! Kamikaze — ennemi suicide qui poursuit le joueur puis explose.
//!
//! Cycle :
//! 1. **pursuing** : poursuit le joueur en continu (`Chase`), animation flammes,
//!    `PlayerDetection` cercle close-range guette le contact rapproché.
//! 2. **counting_down** : déclenché quand le joueur entre dans la zone. Le
//!    kamikaze s'immobilise, clignote rouge, court timer de telegraph.
//! 3. **exploding** : insère `KamikazeExplode`. Le système `kamikaze_explode_system`
//!    spawn une `AreaOfEffect` à la position courante puis despawn le kamikaze.
//!
//! Le kamikaze est **vulnérable** (Health=2) : si le joueur le tue avant qu'il
//! soit en range, il meurt silencieusement (transition outer choice vers
//! `dying`, sans spawn d'AOE). Récompense l'anticipation.

use std::time::Duration;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::behavior::BehaviorBuilder;
use crate::behavior::behavior::BehaviorComponent;
use crate::behavior::choice_list::TransitionMessages;
use crate::enemy::anim_bank::Animation;
use crate::enemy::death::DespawnSelf;
use crate::enemy::enemies::KAMIKAZE;
use crate::enemy::enemy::Enemy;
use crate::enemy::enemy_builder::EnemyBuilder;
use crate::enemy::mine::BlinkRed;
use crate::game_manager::difficulty::{Difficulty, SpawnPosition};
use crate::geometry::shape::Shape;
use crate::item::item::{DropTable, ItemType};
use crate::movement::chase::Chase;
use crate::movement::despawn_off_screen::DespawnOffScreen;
use crate::movement::movements::Movements;
use crate::physic::area_of_effect::spawn_aoe;
use crate::physic::health::Health;
use crate::physic::player_detection::PlayerDetection;

/// Vitesse de poursuite (px/s). Plus lent que le joueur typique → killable
/// au tir, esquivable en restant mobile.
const KAMIKAZE_CHASE_SPEED: f32 = 350.0;
/// Rayon de détection joueur (px) qui déclenche le countdown. Close range.
const KAMIKAZE_DETECTION_RADIUS: f32 = 100.0;
/// Rayon de l'AOE explosion (px). Légèrement plus large que la détection.
const KAMIKAZE_AOE_RADIUS: f32 = 180.0;
/// Durée du countdown avant explosion (secondes). Très court — telegraph
/// minimum, pas d'échappatoire facile.
const KAMIKAZE_COUNTDOWN_DURATION: f32 = 0.3;
/// Durée de vie de l'AOE (secondes). Bref — pas zone denial, juste un
/// gros punch instantané.
const KAMIKAZE_AOE_LIFETIME: f32 = 0.8;
/// Période du clignotement rouge pendant le countdown.
const KAMIKAZE_BLINK_PERIOD: f32 = 0.15;
/// Durée par frame de l'animation des flammes. 15 frames × 0.08s = 1.2s/cycle.
const KAMIKAZE_FRAME_DURATION: f32 = 0.08;
/// Son joué à l'explosion.
const KAMIKAZE_EXPLOSION_SOUND: &str = "audio/sfx/bomb.ogg";

static KAMIKAZE_DROP_TABLE: [(ItemType, f32); 2] =
    [(ItemType::Bomb, 0.10), (ItemType::BonusScore, 0.15)];

/// Marqueur sur l'entité Kamikaze.
#[derive(Component)]
pub struct Kamikaze;

/// Marker inséré par la choice quand le kamikaze doit exploser. Consommé par
/// `kamikaze_explode_system` (spawn AOE + despawn).
#[derive(Component, Clone)]
pub struct KamikazeExplode;

pub struct KamikazeBuilder {
    timer: Timer,
}
impl KamikazeBuilder {
    pub fn new() -> Self {
        Self {
            timer: Timer::new(Duration::ZERO, TimerMode::Once),
        }
    }
}
impl EnemyBuilder for KamikazeBuilder {
    fn get_timer(&mut self) -> &mut Timer {
        &mut self.timer
    }
    fn name(&self) -> &str {
        "kamikaze"
    }
    fn preload_anim(&self) -> HashMap<&str, &str> {
        HashMap::from([("kamikaze", "images/kamikaze")])
    }
    fn spawn(
        &self,
        mut commands: Commands,
        window: &Window,
        _difficulty: &ResMut<Difficulty>,
        spawn_pos: SpawnPosition,
        asset_server: &Res<AssetServer>,
    ) {
        let pos = spawn_pos.resolve(window, KAMIKAZE.config.sprite_size / 2.0);

        // Phase 0 : poursuite continue, PlayerDetection guette.
        let pursuing = BehaviorBuilder::from_component(
            Movements::new().with(Chase::new(KAMIKAZE_CHASE_SPEED)),
        );

        // Phase 1 : immobile + clignote rouge pendant KAMIKAZE_COUNTDOWN_DURATION.
        // `on_complete` pousse "countdown_done" → choice transite vers exploding.
        let counting_down = BehaviorBuilder::first(
            Duration::from_secs_f32(KAMIKAZE_COUNTDOWN_DURATION),
            BehaviorBuilder::multiple()
                .with(BehaviorBuilder::from_component(Movements::new()))
                .with(BehaviorBuilder::from_component(BlinkRed::new(
                    KAMIKAZE_BLINK_PERIOD,
                ))),
        )
        .on_complete("countdown_done");

        // Phase 2 : marker pour le système d'explosion.
        let exploding = BehaviorBuilder::from_component(KamikazeExplode);

        // Pas de transition retour : countdown engagé = explosion garantie
        // (sauf si tué par tirs entretemps → outer choice transite vers dying).
        let alive_choice = BehaviorBuilder::choice()
            .with(pursuing) // 0
            .with(counting_down) // 1
            .with(exploding) // 2
            .add_transition(0, 1, "player_detected")
            .add_transition(1, 2, "countdown_done");

        // Mort silencieuse si tué par tirs (HP=0 → "die" via detect_death).
        let dying = BehaviorBuilder::from_component(DespawnSelf);

        let behavior = BehaviorBuilder::choice()
            .with(alive_choice)
            .with(dying)
            .add_transition(0, 1, "die");

        commands.spawn((
            Sprite {
                image: asset_server.load("images/kamikaze/frame000.png"),
                custom_size: Some(Vec2::splat(KAMIKAZE.config.sprite_size)),
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y, 0.5),
            TransitionMessages::new(),
            Enemy::new(KAMIKAZE),
            Health::new(KAMIKAZE.total_hp),
            Animation::new("kamikaze", Duration::from_secs_f32(KAMIKAZE_FRAME_DURATION)),
            DespawnOffScreen,
            Kamikaze,
            PlayerDetection {
                shape: Shape::Circle(KAMIKAZE_DETECTION_RADIUS),
                on_enter: Some("player_detected"),
                on_exit: None,
                inside: false,
                cooldown_duration: 0.0,
                cooldown_remaining: 0.0,
            },
            BehaviorComponent::new(behavior),
            DropTable {
                drops: &KAMIKAZE_DROP_TABLE,
            },
        ));
    }
}

/// Détecte le marker `KamikazeExplode` : spawn l'AOE à la position courante
/// du kamikaze, joue le son d'explosion, puis despawn le kamikaze.
pub fn kamikaze_explode_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    query: Query<(Entity, &Transform), With<KamikazeExplode>>,
) {
    for (entity, transform) in &query {
        spawn_aoe(
            &mut commands,
            transform.translation,
            Shape::Circle(KAMIKAZE_AOE_RADIUS),
            KAMIKAZE_AOE_LIFETIME,
        );
        commands.spawn((
            AudioPlayer::new(asset_server.load(KAMIKAZE_EXPLOSION_SOUND)),
            PlaybackSettings::DESPAWN,
        ));
        commands.entity(entity).try_despawn();
    }
}
