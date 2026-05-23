//! Kamikaze — ennemi suicide simple : chase + boom au contact ou à HP=0.
//!
//! Comportement minimal : il poursuit le joueur (`Chase`), gagne en vitesse
//! avec l'âge (`KamikazeSpeedRamp`), et explose dans 2 cas seulement :
//! 1. **HP = 0** (tué par les tirs joueur)
//! 2. **Contact physique avec le joueur** (`OverlapEvent` joueur ↔ kamikaze)
//!
//! Les deux cas sont gérés par `kamikaze_force_boom_system` qui insère le
//! marker `KamikazeBoom`. Le hook `on_insert` joue `Sfx::Explosion`, et
//! `kamikaze_boom_system` spawn l'AOE + écrit `DropEvent` + despawn.
//!
//! **Pas d'animation sur le sprite du kamikaze** : sprite statique
//! `kamikaze/chase/frame003.png` (la dernière frame de chase, "flammes
//! armées"). La seule anim qui reste est celle de l'AOE d'explosion
//! (préchargée sous le nom `kamikaze_explosion`).
//!
//! **Pas de PlayerDetection** : pas de phase armée intermédiaire. Le
//! kamikaze est dangereux en permanence dès qu'il touche le joueur.
//!
//! **Rire en loop** : un `AudioPlayer` `KamikazeLaugh` en boucle est
//! attaché en CHILD du kamikaze dès le spawn (`kamikaze_laugh_start_system`
//! sur `Added<Kamikaze>`). Cleanup auto via la cascade parent-enfant quand
//! le kamikaze est despawn (boom, off-screen, etc.) — pas de système stop.

use std::time::Duration;

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::audio::{Sfx, spawn_sfx};
use crate::enemy::death::DespawnSelf;
use crate::enemy::enemies::KAMIKAZE;
use crate::enemy::enemy::Enemy;
use crate::enemy::enemy_builder::EnemyBuilder;
use crate::game_manager::difficulty::{Difficulty, SpawnPosition};
use crate::geometry::shape::Shape;
use crate::item::item::{DropEvent, DropTable, ItemType};
use crate::movement::chase::Chase;
use crate::movement::despawn_off_screen::DespawnOffScreen;
use crate::movement::movements::Movements;
use crate::physic::area_of_effect::spawn_aoe_animated;
use crate::physic::collider::{collider, layers};
use crate::physic::health::Health;
use crate::sprite_orient::FaceMovement;

/// Vitesse de poursuite de base (px/s). Esquivable au début, devient
/// progressivement plus rapide via `KamikazeSpeedRamp`.
const KAMIKAZE_CHASE_SPEED: f32 = 340.0;
/// Bonus de vitesse gagné par seconde de vie (px/s par seconde). Cumulé
/// linéairement avec `KAMIKAZE_CHASE_SPEED`. Plus le kamikaze survit
/// longtemps, plus il devient dangereux — pousse à le prioriser.
const KAMIKAZE_RAMP_RATE: f32 = 120.0;
/// Bonus de vitesse maximum atteint (px/s). Plafond pour éviter la course
/// folle. Atteint après `MAX_BONUS / RAMP_RATE` secondes de vie.
const KAMIKAZE_MAX_BONUS: f32 = 1000.0;
/// Rayon de l'AOE explosion (px).
const KAMIKAZE_AOE_RADIUS: f32 = 180.0;
/// Durée de vie de l'AOE (secondes). Bref — pas zone denial, juste un
/// gros punch instantané. L'animation d'explosion joue sur cette durée.
const KAMIKAZE_AOE_LIFETIME: f32 = 0.4;
/// Taille du sprite de l'AOE (px). Diamètre = 2 × radius pour que
/// l'animation déborde un peu sur la zone d'impact.
const KAMIKAZE_AOE_SPRITE_SIZE: f32 = KAMIKAZE_AOE_RADIUS * 2.0;

static KAMIKAZE_DROP_TABLE: [(ItemType, f32); 3] = [
    (ItemType::Bomb, 0.10),
    (ItemType::BonusScore, 0.15),
    (ItemType::Armor, 0.08),
];

/// Marker sur l'entité Kamikaze.
#[derive(Component)]
pub struct Kamikaze;

/// Inséré pour déclencher l'explosion (HP=0 ou contact joueur). Le hook
/// `on_insert` joue `Sfx::Explosion` ; `kamikaze_boom_system` spawn l'AOE,
/// écrit `DropEvent`, et insère `DespawnSelf`. Une seule fois par entité.
#[derive(Component, Clone)]
#[component(on_insert = play_explosion)]
pub struct KamikazeBoom;

fn play_explosion(mut world: DeferredWorld, _: HookContext) {
    spawn_sfx(&mut world, Sfx::Explosion);
}

/// Composant de speed-ramp : le kamikaze accélère linéairement avec son âge,
/// plafonné à `KAMIKAZE_MAX_BONUS`. Ticked par `kamikaze_speed_ramp_system`.
#[derive(Component)]
pub struct KamikazeSpeedRamp {
    elapsed: f32,
}

impl KamikazeSpeedRamp {
    pub fn new() -> Self {
        Self { elapsed: 0.0 }
    }
}

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
    fn name(&self) -> &'static str {
        "kamikaze"
    }
    fn preload_anim(&self) -> HashMap<&str, &str> {
        // Seule l'anim de l'AOE d'explosion est utilisée. Le sprite du
        // kamikaze lui-même est statique (`chase/frame000.png`).
        HashMap::from([("kamikaze_explosion", "images/kamikaze/explosion")])
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

        // Pas de BehaviorComponent ni TransitionMessages : le kamikaze n'a
        // plus d'états. Il chase jusqu'à exploser (contact ou HP=0), point.
        commands.spawn((
            Sprite {
                // Frame finale de la séquence chase = "flammes armées",
                // utilisée comme sprite statique permanent.
                image: asset_server.load("images/kamikaze/chase/frame003.png"),
                custom_size: Some(Vec2::splat(KAMIKAZE.config.sprite_size)),
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y, 0.5),
            Enemy::new(KAMIKAZE),
            Health::new(KAMIKAZE.total_hp),
            DespawnOffScreen,
            Kamikaze,
            KamikazeSpeedRamp::new(),
            FaceMovement::faces_left(),
            Movements::new().with(Chase::new(KAMIKAZE_CHASE_SPEED)),
            collider(
                Shape::Circle(KAMIKAZE.config.radius),
                layers::ENEMY,
                layers::PLAYER | layers::PLAYER_PROJECTILE,
            ),
            DropTable {
                drops: &KAMIKAZE_DROP_TABLE,
            },
        ));
    }
}

/// Sur `Added<Kamikaze>` : attache un `AudioPlayer` `KamikazeLaugh` en LOOP
/// comme enfant du kamikaze. La relation parent-enfant assure que l'audio
/// est despawn en cascade quand le kamikaze meurt (boom, hors écran, etc.) —
/// pas besoin d'un système stop dédié.
pub fn kamikaze_laugh_start_system(
    mut commands: Commands,
    library: Res<crate::audio::SfxLibrary>,
    query: Query<Entity, Added<Kamikaze>>,
) {
    for kamikaze_entity in &query {
        if let Ok(mut e) = commands.get_entity(kamikaze_entity) {
            e.with_children(|parent| {
                parent.spawn((
                    AudioPlayer::new(library.get(Sfx::KamikazeLaugh)),
                    PlaybackSettings::LOOP,
                ));
            });
        }
    }
}

/// Applique un déplacement additionnel vers le joueur dont la magnitude
/// grandit avec l'âge du kamikaze (`bonus = age * RAMP_RATE`, plafonné à
/// `MAX_BONUS`). S'additionne au `Chase` standard.
pub fn kamikaze_speed_ramp_system(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut KamikazeSpeedRamp)>,
    player_tf: Single<
        &Transform,
        (
            With<crate::player::player::Player>,
            Without<KamikazeSpeedRamp>,
        ),
    >,
) {
    let dt = time.delta_secs();
    let player_pos = player_tf.translation.xy();

    for (mut tf, mut ramp) in &mut query {
        ramp.elapsed += dt;
        let bonus = (ramp.elapsed * KAMIKAZE_RAMP_RATE).min(KAMIKAZE_MAX_BONUS);
        let dir = (player_pos - tf.translation.xy()).normalize_or_zero();
        let delta = dir * bonus * dt;
        tf.translation.x += delta.x;
        tf.translation.y += delta.y;
    }
}

/// Insère `KamikazeBoom` dans 2 cas :
/// - **HP = 0** (tué par les tirs joueur) — check direct sur `Health`
/// - **Contact physique avec le joueur** — via `OverlapEvent`
///
/// `Without<KamikazeBoom>` empêche le double-déclenchement (idempotent).
pub fn kamikaze_force_boom_system(
    mut commands: Commands,
    mut events: MessageReader<crate::physic::collider::OverlapEvent>,
    kamikaze_hp_q: Query<(Entity, &Health), (With<Kamikaze>, Without<KamikazeBoom>)>,
    kamikaze_marker_q: Query<(), (With<Kamikaze>, Without<KamikazeBoom>)>,
) {
    // 1. HP=0 → boom (check direct, pas via collision)
    for (entity, health) in &kamikaze_hp_q {
        if health.is_dead() {
            if let Ok(mut e) = commands.get_entity(entity) {
                // `try_insert` : safe si l'entité est despawn entre `get_entity`
                // et le flush (ex: tuée la même frame par une bombe joueur,
                // qui despawn directement les kamikazes via `bomb_apply_damage`).
                e.try_insert(KamikazeBoom);
            }
        }
    }
    // 2. Overlap player ↔ kamikaze → boom
    for ev in events.read() {
        let Some((_, kam_e)) = ev.pick_pair(layers::PLAYER, layers::ENEMY) else { continue };
        // Filtre : c'est un kamikaze (pas un autre Enemy) qui n'a pas déjà KamikazeBoom
        if !kamikaze_marker_q.contains(kam_e) { continue; }
        if let Ok(mut e) = commands.get_entity(kam_e) {
            e.try_insert(KamikazeBoom);
        }
    }
}

/// Détecte `Added<KamikazeBoom>` : spawn l'AOE animée, écrit le `DropEvent`
/// pour faire tomber d'éventuels items (`KAMIKAZE_DROP_TABLE`), puis insère
/// `DespawnSelf` pour despawn proprement en PostUpdate. Le son `Explosion`
/// est joué par le hook `on_insert` sur `KamikazeBoom`.
///
/// **Pourquoi écrire `DropEvent` ici** : sans `BehaviorComponent` ni
/// `TransitionMessages`, `detect_death` (qui exige ces composants dans son
/// query) ne se déclenche jamais pour le kamikaze. On émet donc le drop
/// nous-mêmes au moment du boom.
pub fn kamikaze_boom_system(
    mut commands: Commands,
    anim_bank: Res<crate::enemy::anim_bank::AnimBank>,
    mut drop_events: MessageWriter<DropEvent>,
    query: Query<(Entity, &Transform, Option<&DropTable>), Added<KamikazeBoom>>,
) {
    for (entity, transform, drop_table) in &query {
        spawn_aoe_animated(
            &mut commands,
            &anim_bank,
            transform.translation,
            Shape::Circle(KAMIKAZE_AOE_RADIUS),
            KAMIKAZE_AOE_LIFETIME,
            "kamikaze_explosion",
            KAMIKAZE_AOE_SPRITE_SIZE,
        );
        if let Some(table) = drop_table {
            drop_events.write(DropEvent {
                position: transform.translation,
                table: table.drops,
            });
        }
        // DespawnSelf au lieu de try_despawn direct : évite la race avec les
        // commandes du behavior tree (cf. collision.rs pour les détails).
        if let Ok(mut e) = commands.get_entity(entity) {
            e.try_insert(DespawnSelf);
        }
    }
}
