//! Gatling — ennemi utilisant le framework `enemy.rs`.
//!
//! Machine à état : Entering (3s, descend 128px) → Active(0) → Dying → Dead
//! En Active : immobile (idle).

use crate::enemies::GATLING;
use crate::enemy::{Enemy, EnemyState, PatternIndex, PatternTimer};
use crate::item::{DropTable, ItemType};
use crate::pause::not_paused;
use crate::state::GameState;
use bevy::prelude::*;

pub struct GatlingPlugin;

impl Plugin for GatlingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                spawn_gatlings_oneshot,
                gatling_entering,
            )
                .run_if(in_state(GameState::Playing))
                .run_if(not_paused),
        );
    }
}

// ─── Constantes ─────────────────────────────────────────────────────

/// Durée de la phase Entering (secondes).
const GATLING_ENTERING_DURATION: f32 = 3.0;
/// Distance parcourue pendant l'Entering (pixels).
const GATLING_ENTERING_DISTANCE: f32 = 128.0;
/// Taille du sprite (pixels).
const GATLING_SPRITE_SIZE: f32 = 128.0;

/// Drop table : 10% bombe, 15% bonus score.
static GATLING_DROP_TABLE: [(ItemType, f32); 2] =
    [(ItemType::Bomb, 0.10), (ItemType::BonusScore, 0.15)];

// ─── Composants ─────────────────────────────────────────────────────

/// Marqueur pour identifier les Gatling parmi les Enemy.
#[derive(Component)]
pub struct GatlingMarker;

/// Position Y de départ, enregistrée au spawn pour l'animation Entering.
#[derive(Component)]
struct GatlingStartY(f32);

// ─── Spawn one-shot (via spawn_requests) ────────────────────────────

/// Consomme les requêtes "gatling" dans `difficulty.spawn_requests`.
fn spawn_gatlings_oneshot(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut difficulty: ResMut<crate::difficulty::Difficulty>,
    windows: Query<&Window>,
) {
    let Some(pos) = difficulty
        .spawn_requests
        .iter()
        .position(|(name, _, _)| *name == "gatling")
    else {
        return;
    };
    let (_name, count, spawn_pos) = difficulty.spawn_requests.remove(pos);

    let window = windows.single();
    for _ in 0..count {
        spawn_one_gatling(&mut commands, &asset_server, window, spawn_pos);
    }
}

/// Spawne un seul Gatling à la position donnée.
fn spawn_one_gatling(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    window: &Window,
    spawn_pos: crate::difficulty::SpawnPosition,
) {
    let pos = spawn_pos.resolve(window, 60.0);
    let phase = &GATLING.phases[0];

    commands.spawn((
        SpriteBundle {
            texture: asset_server.load("images/gatling/frame000.png"),
            sprite: Sprite {
                custom_size: Some(Vec2::splat(GATLING_SPRITE_SIZE)),
                ..default()
            },
            transform: Transform::from_xyz(pos.x, pos.y, 0.5),
            ..default()
        },
        Enemy {
            health: phase.health,
            max_health: phase.health,
            state: EnemyState::Entering,
            radius: GATLING.radius,
            sprite_size: GATLING.sprite_size,
            anim_timer: Timer::from_seconds(GATLING_ENTERING_DURATION, TimerMode::Once),
            phases: GATLING.phases,
            death_duration: GATLING.death_duration,
            death_shake_max: GATLING.death_shake_max,
            hit_sound: GATLING.hit_sound,
            death_explosion_sound: GATLING.death_explosion_sound,
            hit_flash_color: None,
        },
        GatlingMarker,
        GatlingStartY(pos.y),
        PatternIndex(0),
        PatternTimer(Timer::from_seconds(0.0, TimerMode::Once)),
        DropTable {
            drops: &GATLING_DROP_TABLE,
        },
    ));
}

// ─── Entering : descente de 128px en 3s ─────────────────────────────

fn gatling_entering(
    time: Res<Time>,
    mut query: Query<(&mut Enemy, &mut Transform, &GatlingStartY), With<GatlingMarker>>,
) {
    for (mut enemy, mut transform, start_y) in query.iter_mut() {
        if enemy.state != EnemyState::Entering {
            continue;
        }

        enemy.anim_timer.tick(time.delta());
        let progress = enemy.anim_timer.fraction();

        // Ease-out quadratique
        let eased = 1.0 - (1.0 - progress).powi(2);

        transform.translation.y = start_y.0 - GATLING_ENTERING_DISTANCE * eased;

        if enemy.anim_timer.finished() {
            transform.translation.y = start_y.0 - GATLING_ENTERING_DISTANCE;
            enemy.state = EnemyState::Active(0);
        }
    }
}
