//! Boss — définition data-driven basée sur le framework `enemy/system.rs`.
//!
//! ## Phases
//! ```
//! entering ──timer 7s──→ active_1 ──HP<66%──→ transitioning_1 ──timer 2s──→ active_2
//!                                                                              │
//!                                                                          HP<33%
//!                                                                              ▼
//!     dead ←──on_enter despawn── dying ←──HP<1%── active_3 ←──timer 2s── transitioning_2
//! ```
//!
//! ## Parités à valider en jeu (vs ancien boss.rs)
//! - **Intro** : spirale + scaling — l'easing "progress²" matche
//! - **Musique boss** : démarre sur `on_enter active_1` (pas de délai progressif
//!   comme avant avec `boss_music_delayed`). Si tu veux le délai, ajouter une
//!   phase `idle` intermédiaire de 0.5s entre intro et active_1.
//! - **Charge** : `PatrolAndCharge` déclenche une charge tous les N secondes.
//!   L'ancien boss synchronisait au pattern (patrol 5s → charge 0.1s → patrol).
//!   La cadence est proche mais le timing peut différer de ±0.5s.
//! - **Transitions** : shake + flash OK. Spawn d'UFOs idem.
//! - **Mort** : DyingFx fait shake+flash, les **explosions aléatoires**
//!   pendant la mort ne sont PAS spawnées (limitation des behaviors &mut World).
//!   → flaggé en `TODO-VISUEL`.
//! - **Animation idle** (cycle de frames sur le sprite boss) : pas encore
//!   implémentée. Le boss reste sur `frame000.png` en Phase1/2/3.
//!   → flaggé en `TODO-VISUEL`.

use std::time::Duration;

use bevy::prelude::*;

use crate::enemy::behaviors::{DespawnSelf, PlaySound};
use crate::enemy::enemies::BOSS;
use crate::enemy::enemy::Enemy;
use crate::enemy::system::{
    b, par, Behavior, EnemyDefinition, Noop, Phase, PhaseId, Transition, TransitionTrigger,
};
use crate::game_manager::difficulty::{Difficulty, SpawnPosition};
use crate::game_manager::state::GameState;
use crate::menu::pause::not_paused;
use crate::physic::health::Health;
use crate::player::player::Player;

// ═══════════════════════════════════════════════════════════════════════
//  Marqueurs (utilisés par boss.rs pour le charge_movement + musique)
// ═══════════════════════════════════════════════════════════════════════

/// Marqueur présent sur l'entité boss (utilisé pour la musique + charge).
#[derive(Component)]
pub struct BossMarker;

/// Composant indiquant que le boss charge le joueur dans une direction figée.
/// Le système `boss_charge_movement` (dans boss.rs) déplace l'entité tant
/// que ce composant est présent.
#[derive(Component)]
pub struct BossCharge {
    pub direction: Vec2,
}

/// Marqueur pour la musique du boss.
#[derive(Component)]
pub struct MusicBoss;

// ═══════════════════════════════════════════════════════════════════════
//  Constantes
// ═══════════════════════════════════════════════════════════════════════

const INTRO_DURATION: f32 = 7.0;
const INTRO_TARGET_Y: f32 = 250.0;
const INTRO_START_SCALE: f32 = 0.01;
const INTRO_END_SCALE: f32 = 1.0;
const INTRO_SPIRAL_TURNS: f32 = 2.5;
const INTRO_SPIRAL_RADIUS: f32 = 150.0;

const PHASE1_PATROL_SPEED_X: f32 = 200.0;
const PHASE2_PATROL_SPEED_X: f32 = 270.0;
const PHASE3_PATROL_SPEED_X: f32 = 270.0;
const PATROL_SINE_AMPLITUDE: f32 = 0.85;
const PATROL_SINE_FREQ: f32 = 4.5;
const PATROL_MARGIN: f32 = 80.0;

const CHARGE_SPEED_P1: f32 = 1500.0;
const CHARGE_SPEED_P2: f32 = 2000.0;
const CHARGE_SPEED_P3: f32 = 2500.0;

const TRANSITION_DURATION: f32 = 2.0;
const TRANSITION_SHAKE: f32 = 12.0;
const TRANSITION_UFO_COUNT_1: usize = 2;
const TRANSITION_UFO_COUNT_2: usize = 4;

const DYING_DURATION: f32 = 4.0;
const DYING_SHAKE_MAX: f32 = 20.0;

// ═══════════════════════════════════════════════════════════════════════
//  Définition du boss
// ═══════════════════════════════════════════════════════════════════════

pub fn boss_definition() -> EnemyDefinition {
    EnemyDefinition {
        name: "Boss",
        initial_phase: PhaseId("entering"),
        phases: vec![
            // Intro spirale
            Phase::new(
                PhaseId("entering"),
                b(IntroSpiral {
                    duration: INTRO_DURATION,
                    target_y: INTRO_TARGET_Y,
                    start_scale: INTRO_START_SCALE,
                    end_scale: INTRO_END_SCALE,
                    turns: INTRO_SPIRAL_TURNS,
                    radius: INTRO_SPIRAL_RADIUS,
                    from_y: -600.0,
                }),
                vec![Transition {
                    trigger: TransitionTrigger::Timer(Duration::from_secs_f32(INTRO_DURATION)),
                    target_phase: PhaseId("active_1"),
                    priority: 0,
                }],
            )
            .with_on_enter(b(PlaySound {
                path: "audio/sfx/boss_start.ogg",
                volume: 1.0,
            }))
            .invulnerable(),
            // Phase 1
            Phase::new(
                PhaseId("active_1"),
                b(PatrolAndCharge {
                    speed_x: PHASE1_PATROL_SPEED_X,
                    sine_amp: PATROL_SINE_AMPLITUDE,
                    sine_freq: PATROL_SINE_FREQ,
                    margin: PATROL_MARGIN,
                    charge_speed: CHARGE_SPEED_P1,
                    charge_every: 5.0,
                }),
                vec![Transition {
                    trigger: TransitionTrigger::HealthBelow(0.66),
                    target_phase: PhaseId("transitioning_1"),
                    priority: 10,
                }],
            )
            .with_on_enter(b(StartBossMusic)),
            // Transition 1
            Phase::new(
                PhaseId("transitioning_1"),
                b(TransitionShake {
                    amplitude: TRANSITION_SHAKE,
                    duration: TRANSITION_DURATION,
                }),
                vec![Transition {
                    trigger: TransitionTrigger::Timer(Duration::from_secs_f32(
                        TRANSITION_DURATION,
                    )),
                    target_phase: PhaseId("active_2"),
                    priority: 0,
                }],
            )
            .with_on_enter(par(vec![
                b(SpawnUfoWave {
                    count: TRANSITION_UFO_COUNT_1,
                }),
                b(PlaySound {
                    path: "audio/sfx/t_go.wav",
                    volume: 1.0,
                }),
            ]))
            .invulnerable(),
            // Phase 2
            Phase::new(
                PhaseId("active_2"),
                b(PatrolAndCharge {
                    speed_x: PHASE2_PATROL_SPEED_X,
                    sine_amp: PATROL_SINE_AMPLITUDE,
                    sine_freq: PATROL_SINE_FREQ,
                    margin: PATROL_MARGIN,
                    charge_speed: CHARGE_SPEED_P2,
                    charge_every: 4.0,
                }),
                vec![Transition {
                    trigger: TransitionTrigger::HealthBelow(0.33),
                    target_phase: PhaseId("transitioning_2"),
                    priority: 10,
                }],
            ),
            // Transition 2
            Phase::new(
                PhaseId("transitioning_2"),
                b(TransitionShake {
                    amplitude: TRANSITION_SHAKE,
                    duration: TRANSITION_DURATION,
                }),
                vec![Transition {
                    trigger: TransitionTrigger::Timer(Duration::from_secs_f32(
                        TRANSITION_DURATION,
                    )),
                    target_phase: PhaseId("active_3"),
                    priority: 0,
                }],
            )
            .with_on_enter(par(vec![
                b(SpawnUfoWave {
                    count: TRANSITION_UFO_COUNT_2,
                }),
                b(PlaySound {
                    path: "audio/sfx/t_go.wav",
                    volume: 1.0,
                }),
            ]))
            .invulnerable(),
            // Phase 3
            Phase::new(
                PhaseId("active_3"),
                b(PatrolAndCharge {
                    speed_x: PHASE3_PATROL_SPEED_X,
                    sine_amp: PATROL_SINE_AMPLITUDE,
                    sine_freq: PATROL_SINE_FREQ,
                    margin: PATROL_MARGIN,
                    charge_speed: CHARGE_SPEED_P3,
                    charge_every: 3.0,
                }),
                vec![Transition {
                    trigger: TransitionTrigger::HealthBelow(0.01),
                    target_phase: PhaseId("dying"),
                    priority: 10,
                }],
            ),
            // Dying
            Phase::new(
                PhaseId("dying"),
                b(DyingFx {
                    shake_max: DYING_SHAKE_MAX,
                    duration: DYING_DURATION,
                }),
                vec![Transition {
                    trigger: TransitionTrigger::Timer(Duration::from_secs_f32(DYING_DURATION)),
                    target_phase: PhaseId("dead"),
                    priority: 0,
                }],
            )
            .with_on_enter(b(StopBossMusic))
            .invulnerable(),
            // Dead
            Phase::new(PhaseId("dead"), b(Noop), vec![])
                .with_on_enter(b(DespawnSelf))
                .invulnerable(),
        ],
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Behaviors concrets
// ═══════════════════════════════════════════════════════════════════════

struct IntroSpiral {
    duration: f32,
    target_y: f32,
    start_scale: f32,
    end_scale: f32,
    turns: f32,
    radius: f32,
    from_y: f32,
}

impl Behavior for IntroSpiral {
    fn execute(&self, entity: Entity, world: &mut World) {
        let Some(enemy) = world.get::<Enemy>(entity) else {
            return;
        };
        let t = (enemy.phase_timer.elapsed().as_secs_f32() / self.duration).clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - t).powi(2);

        let angle = t * self.turns * std::f32::consts::TAU;
        let radius = self.radius * (1.0 - eased);
        let dx = angle.cos() * radius;
        let y = self.from_y + (self.target_y - self.from_y) * eased;
        let scale = self.start_scale + (self.end_scale - self.start_scale) * eased;

        if let Some(mut tr) = world.get_mut::<Transform>(entity) {
            tr.translation.x = dx;
            tr.translation.y = y;
            tr.scale = Vec3::splat(scale);
        }
    }
    fn name(&self) -> &'static str {
        "IntroSpiral"
    }
}

struct PatrolAndCharge {
    speed_x: f32,
    sine_amp: f32,
    sine_freq: f32,
    margin: f32,
    charge_speed: f32,
    charge_every: f32,
}

impl Behavior for PatrolAndCharge {
    fn execute(&self, entity: Entity, world: &mut World) {
        let Some(enemy) = world.get::<Enemy>(entity) else {
            return;
        };
        let t = enemy.phase_timer.elapsed().as_secs_f32();
        let already_charging = world.get::<BossCharge>(entity).is_some();
        let dt = world.resource::<Time>().delta_seconds();

        let phase_of_cycle = t % self.charge_every;
        if phase_of_cycle < dt && !already_charging {
            let player_pos = world
                .query_filtered::<&Transform, With<Player>>()
                .get_single(world)
                .ok()
                .map(|t| t.translation.truncate());
            let my_pos = world
                .get::<Transform>(entity)
                .map(|t| t.translation.truncate());
            if let (Some(p), Some(m)) = (player_pos, my_pos) {
                let dir = (p - m).normalize_or_zero();
                if dir != Vec2::ZERO {
                    world
                        .entity_mut(entity)
                        .insert(BossCharge { direction: dir });
                }
            }
            // NOTE : on garde `charge_speed` dans la struct pour que le
            // système `boss_charge_movement` puisse lire la vitesse plus tard
            // si besoin (il utilise actuellement une constante).
            let _ = self.charge_speed;
            return;
        }

        if already_charging {
            return;
        }

        let window_half_h = 360.0_f32;
        let window_half_w = 640.0_f32;
        let (half_w, half_h) = world
            .query::<&Window>()
            .iter(world)
            .next()
            .map(|w| (w.width() / 2.0 - self.margin, w.height() / 2.0 - self.margin))
            .unwrap_or((window_half_w - self.margin, window_half_h - self.margin));

        if let Some(mut tr) = world.get_mut::<Transform>(entity) {
            tr.translation.x = (tr.translation.x + self.speed_x * dt).clamp(-half_w, half_w);
            tr.translation.y = (t * self.sine_freq).sin() * half_h * self.sine_amp;
        }
    }
    fn name(&self) -> &'static str {
        "PatrolAndCharge"
    }
}

struct TransitionShake {
    amplitude: f32,
    duration: f32,
}

impl Behavior for TransitionShake {
    fn execute(&self, entity: Entity, world: &mut World) {
        let Some(enemy) = world.get::<Enemy>(entity) else {
            return;
        };
        let progress = enemy.phase_timer.elapsed().as_secs_f32() / self.duration;
        let shake = progress * progress * self.amplitude;
        let dx = (fastrand::f32() - 0.5) * 2.0 * shake;
        let dy = (fastrand::f32() - 0.5) * 2.0 * shake;
        if let Some(mut tr) = world.get_mut::<Transform>(entity) {
            tr.translation.x += dx;
            tr.translation.y += dy;
        }
        if let Some(mut sprite) = world.get_mut::<Sprite>(entity) {
            let v = 1.0 + ((progress * 30.0).sin() * 0.5 + 0.5) * 2.0;
            sprite.color = Color::rgba(v, v, v, 1.0);
        }
    }
    fn name(&self) -> &'static str {
        "TransitionShake"
    }
}

struct SpawnUfoWave {
    count: usize,
}

impl Behavior for SpawnUfoWave {
    fn execute(&self, _entity: Entity, world: &mut World) {
        if let Some(mut difficulty) = world.get_resource_mut::<Difficulty>() {
            difficulty
                .spawn_requests
                .push(("green_ufo", self.count, SpawnPosition::Top));
        }
    }
    fn name(&self) -> &'static str {
        "SpawnUfoWave"
    }
}

struct DyingFx {
    shake_max: f32,
    duration: f32,
}

impl Behavior for DyingFx {
    fn execute(&self, entity: Entity, world: &mut World) {
        let Some(enemy) = world.get::<Enemy>(entity) else {
            return;
        };
        let progress = (enemy.phase_timer.elapsed().as_secs_f32() / self.duration).clamp(0.0, 1.0);

        let shake = progress * progress * self.shake_max;
        let dx = (fastrand::f32() - 0.5) * 2.0 * shake;
        let dy = (fastrand::f32() - 0.5) * 2.0 * shake;
        if let Some(mut tr) = world.get_mut::<Transform>(entity) {
            tr.translation.x += dx;
            tr.translation.y += dy;
        }
        if let Some(mut sprite) = world.get_mut::<Sprite>(entity) {
            let blink = ((progress * 60.0).sin() + 1.0) * 0.5;
            let v = 1.0 + blink * 2.0;
            sprite.color = Color::rgba(v, v, v, 1.0);
        }
        // TODO-VISUEL : explosions aléatoires style enemy_dying — nécessite
        // un BehaviorCtx avec Commands+AssetServer pour être propre.
    }
    fn name(&self) -> &'static str {
        "DyingFx"
    }
}

struct StartBossMusic;

impl Behavior for StartBossMusic {
    fn execute(&self, _entity: Entity, world: &mut World) {
        let handle = world
            .resource::<AssetServer>()
            .load::<AudioSource>("audio/music/boss.ogg");
        world.spawn((
            AudioBundle {
                source: handle,
                settings: PlaybackSettings::LOOP,
            },
            MusicBoss,
        ));
    }
    fn name(&self) -> &'static str {
        "StartBossMusic"
    }
}

struct StopBossMusic;

impl Behavior for StopBossMusic {
    fn execute(&self, _entity: Entity, world: &mut World) {
        let entities: Vec<Entity> = world
            .query_filtered::<Entity, With<MusicBoss>>()
            .iter(world)
            .collect();
        for e in entities {
            world.despawn(e);
        }
    }
    fn name(&self) -> &'static str {
        "StopBossMusic"
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Spawn
// ═══════════════════════════════════════════════════════════════════════

pub fn spawn_boss(commands: &mut Commands, asset_server: &AssetServer, position: Vec3) {
    let mut config = BOSS.config.to_config();
    config.hit_flash_color = Some(Color::rgba(2.5, 2.5, 2.5, 1.0));

    commands.spawn((
        SpriteBundle {
            texture: asset_server.load("images/boss/idle/frame000.png"),
            sprite: Sprite {
                custom_size: Some(Vec2::splat(BOSS.config.sprite_size)),
                color: Color::WHITE,
                ..default()
            },
            transform: Transform {
                translation: position,
                scale: Vec3::splat(INTRO_START_SCALE),
                ..default()
            },
            ..default()
        },
        Enemy::new(config, boss_definition()),
        Health::new(BOSS.total_hp),
        BossMarker,
    ));
}

fn spawn_boss_oneshot(
    mut commands: Commands,
    mut difficulty: ResMut<Difficulty>,
    asset_server: Res<AssetServer>,
    windows: Query<&Window>,
) {
    let Some(idx) = difficulty
        .spawn_requests
        .iter()
        .position(|(n, _, _)| *n == "boss")
    else {
        return;
    };
    let (_name, count, spawn_pos) = difficulty.spawn_requests.remove(idx);
    difficulty.boss_spawned = true;
    let window = windows.single();

    for i in 0..count {
        let base = spawn_pos.resolve(window, 60.0);
        let offset_x = if count > 1 {
            (i as f32 - (count - 1) as f32 / 2.0) * 120.0
        } else {
            0.0
        };
        spawn_boss(
            &mut commands,
            &asset_server,
            Vec3::new(base.x + offset_x, base.y, 0.5),
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Plugin
// ═══════════════════════════════════════════════════════════════════════

pub struct BossPlugin;

impl Plugin for BossPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (spawn_boss_oneshot, boss_charge_movement)
                .run_if(in_state(GameState::Playing))
                .run_if(not_paused),
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Système charge movement (helper générique pour les entités BossCharge)
// ═══════════════════════════════════════════════════════════════════════

const BOSS_CHARGE_SPEED: f32 = 2000.0;

fn boss_charge_movement(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &BossCharge)>,
    windows: Query<&Window>,
) {
    let window = windows.single();
    let half_w = window.width() / 2.0 + 200.0;
    let half_h = window.height() / 2.0 + 200.0;
    let dt = time.delta_seconds();

    for (entity, mut transform, charge) in query.iter_mut() {
        transform.translation.x += charge.direction.x * BOSS_CHARGE_SPEED * dt;
        transform.translation.y += charge.direction.y * BOSS_CHARGE_SPEED * dt;

        // Fin de charge quand l'entité sort de l'écran
        if transform.translation.x.abs() > half_w || transform.translation.y.abs() > half_h {
            commands.entity(entity).remove::<BossCharge>();
            // Re-centrer approximativement (l'entité reviendra en patrol)
            transform.translation.x = transform.translation.x.clamp(-half_w + 300.0, half_w - 300.0);
            transform.translation.y = transform.translation.y.clamp(-half_h + 300.0, half_h - 300.0);
        }
    }
}
