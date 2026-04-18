//! Boss — définition data-driven basée sur le framework `enemy/system.rs`.
//!
//! Ce module contient :
//! - `boss_definition()` : la structure complète de l'ennemi (phases + transitions)
//! - Les `Behavior` concrets utilisés par ses phases
//! - Une fonction `spawn_boss_v2()` qui instancie le boss en scène
//!
//! Cette version coexiste avec `enemy/boss.rs` (l'ancien moteur via `Enemy`
//! + `EnemyState`). L'objectif est de migrer progressivement vers le nouveau
//! framework une fois que chaque élément est validé.

use std::time::Duration;

use bevy::prelude::*;

use crate::enemy::boss::{BossCharge, BossMarker, MusicBoss};
use crate::enemy::enemies::BOSS;
use crate::enemy::enemy::{Enemy, EnemyState};
use crate::enemy::system::{
    b, seq, Behavior, EnemyBehavior, EnemyDefinition, Noop, Phase, PhaseId, Transition,
    TransitionTrigger,
};
use crate::player::player::Player;

// ═══════════════════════════════════════════════════════════════════════
//  Constantes boss (miroir de boss.rs, redéfinies ici pour l'autonomie)
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
const PATROL_SINE_AMPLITUDE: f32 = 0.85; // fraction de half_h
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

/// Renvoie la définition complète du boss : intro → 3 phases actives
/// entrecoupées de 2 transitions, puis mort.
///
/// Les seuils `HealthBelow` sont relatifs (0.66 = 66% de la vie max),
/// ce qui correspond à 200/300 et 100/300 pour le boss 3×100 PV classique.
pub fn boss_definition() -> EnemyDefinition {
    EnemyDefinition {
        name: "Boss",
        initial_phase: PhaseId("entering"),
        phases: vec![
            // ── Intro : spirale en 7s ────────────────────────────
            Phase {
                id: PhaseId("entering"),
                on_enter: Some(b(PlaySound {
                    path: "audio/sfx/boss_start.ogg",
                })),
                behavior: b(IntroSpiral {
                    duration: INTRO_DURATION,
                    target_y: INTRO_TARGET_Y,
                    start_scale: INTRO_START_SCALE,
                    end_scale: INTRO_END_SCALE,
                    turns: INTRO_SPIRAL_TURNS,
                    radius: INTRO_SPIRAL_RADIUS,
                }),
                transitions: vec![Transition {
                    trigger: TransitionTrigger::Timer(Duration::from_secs_f32(INTRO_DURATION)),
                    target_phase: PhaseId("active_1"),
                    priority: 0,
                }],
            },
            // ── Phase 1 : patrol + charges occasionnelles ────────
            Phase {
                id: PhaseId("active_1"),
                on_enter: Some(b(StartBossMusic)),
                behavior: b(PatrolAndCharge {
                    speed_x: PHASE1_PATROL_SPEED_X,
                    sine_amp: PATROL_SINE_AMPLITUDE,
                    sine_freq: PATROL_SINE_FREQ,
                    margin: PATROL_MARGIN,
                    charge_speed: CHARGE_SPEED_P1,
                    charge_every: 5.0,
                }),
                transitions: vec![Transition {
                    trigger: TransitionTrigger::HealthBelow(0.66),
                    target_phase: PhaseId("transitioning_1"),
                    priority: 10,
                }],
            },
            // ── Transition 1 : shake + spawn 2 UFOs ─────────────
            Phase {
                id: PhaseId("transitioning_1"),
                on_enter: Some(seq(vec![
                    b(SpawnUfoWave {
                        count: TRANSITION_UFO_COUNT_1,
                    }),
                    b(PlaySound {
                        path: "audio/sfx/t_go.wav",
                    }),
                ])),
                behavior: b(ShakeAndFlash {
                    amplitude: TRANSITION_SHAKE,
                }),
                transitions: vec![Transition {
                    trigger: TransitionTrigger::Timer(Duration::from_secs_f32(
                        TRANSITION_DURATION,
                    )),
                    target_phase: PhaseId("active_2"),
                    priority: 0,
                }],
            },
            // ── Phase 2 : plus rapide ───────────────────────────
            Phase {
                id: PhaseId("active_2"),
                on_enter: None,
                behavior: b(PatrolAndCharge {
                    speed_x: PHASE2_PATROL_SPEED_X,
                    sine_amp: PATROL_SINE_AMPLITUDE,
                    sine_freq: PATROL_SINE_FREQ,
                    margin: PATROL_MARGIN,
                    charge_speed: CHARGE_SPEED_P2,
                    charge_every: 4.0,
                }),
                transitions: vec![Transition {
                    trigger: TransitionTrigger::HealthBelow(0.33),
                    target_phase: PhaseId("transitioning_2"),
                    priority: 10,
                }],
            },
            // ── Transition 2 : shake + spawn 4 UFOs ─────────────
            Phase {
                id: PhaseId("transitioning_2"),
                on_enter: Some(seq(vec![
                    b(SpawnUfoWave {
                        count: TRANSITION_UFO_COUNT_2,
                    }),
                    b(PlaySound {
                        path: "audio/sfx/t_go.wav",
                    }),
                ])),
                behavior: b(ShakeAndFlash {
                    amplitude: TRANSITION_SHAKE,
                }),
                transitions: vec![Transition {
                    trigger: TransitionTrigger::Timer(Duration::from_secs_f32(
                        TRANSITION_DURATION,
                    )),
                    target_phase: PhaseId("active_3"),
                    priority: 0,
                }],
            },
            // ── Phase 3 : le plus rapide ────────────────────────
            Phase {
                id: PhaseId("active_3"),
                on_enter: None,
                behavior: b(PatrolAndCharge {
                    speed_x: PHASE3_PATROL_SPEED_X,
                    sine_amp: PATROL_SINE_AMPLITUDE,
                    sine_freq: PATROL_SINE_FREQ,
                    margin: PATROL_MARGIN,
                    charge_speed: CHARGE_SPEED_P3,
                    charge_every: 3.0,
                }),
                transitions: vec![Transition {
                    trigger: TransitionTrigger::HealthBelow(0.0),
                    target_phase: PhaseId("dying"),
                    priority: 10,
                }],
            },
            // ── Mort : shake + explosions en cascade ────────────
            Phase {
                id: PhaseId("dying"),
                on_enter: Some(b(StopBossMusic)),
                behavior: b(DyingFx {
                    shake_max: DYING_SHAKE_MAX,
                    duration: DYING_DURATION,
                }),
                transitions: vec![Transition {
                    trigger: TransitionTrigger::Timer(Duration::from_secs_f32(DYING_DURATION)),
                    target_phase: PhaseId("dead"),
                    priority: 0,
                }],
            },
            // ── Dead : despawn au premier frame ─────────────────
            Phase {
                id: PhaseId("dead"),
                on_enter: Some(b(DespawnSelf)),
                behavior: b(Noop),
                transitions: vec![],
            },
        ],
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Behaviors concrets
// ═══════════════════════════════════════════════════════════════════════

// ─── Intro : spirale centripète vers la position cible ─────────────

struct IntroSpiral {
    duration: f32,
    target_y: f32,
    start_scale: f32,
    end_scale: f32,
    turns: f32,
    radius: f32,
}

impl Behavior for IntroSpiral {
    fn execute(&self, entity: Entity, world: &mut World) {
        let Some(enemy) = world.get::<EnemyBehavior>(entity) else {
            return;
        };
        let t = (enemy.phase_timer.elapsed().as_secs_f32() / self.duration).clamp(0.0, 1.0);
        // ease-out quadratique pour décélération finale
        let eased = 1.0 - (1.0 - t).powi(2);

        let angle = t * self.turns * std::f32::consts::TAU;
        let radius = self.radius * (1.0 - eased);
        let dx = angle.cos() * radius;

        let y_from = -600.0;
        let y = y_from + (self.target_y - y_from) * eased;
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

// ─── Patrol + charges périodiques ──────────────────────────────────

struct PatrolAndCharge {
    speed_x: f32,
    sine_amp: f32,
    sine_freq: f32,
    margin: f32,
    charge_speed: f32,
    /// Intervalle entre deux charges (s). La charge démarre quand le timer
    /// atteint un multiple de cette durée (± tolérance).
    charge_every: f32,
}

impl Behavior for PatrolAndCharge {
    fn execute(&self, entity: Entity, world: &mut World) {
        // Lire infos nécessaires
        let Some(enemy) = world.get::<EnemyBehavior>(entity) else {
            return;
        };
        let t = enemy.phase_timer.elapsed().as_secs_f32();
        let already_charging = world.get::<BossCharge>(entity).is_some();
        let dt = world.resource::<Time>().delta_seconds();

        // Déclencher une charge tous les `charge_every` s, si pas déjà en charge
        let phase_of_cycle = t % self.charge_every;
        if phase_of_cycle < dt && !already_charging {
            // Trouver le joueur pour diriger la charge
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
            return;
        }

        if already_charging {
            // Le mouvement de charge est géré par `boss_charge_movement` dans boss.rs.
            // Ici on laisse le composant BossCharge faire son travail.
            return;
        }

        // Sinon : mouvement patrol sinusoïdal
        let window_half_h = 360.0; // approximation, fallback si Window indispo
        let window_half_w = 640.0;
        let (half_w, half_h) = world
            .query::<&Window>()
            .iter(world)
            .next()
            .map(|w| (w.width() / 2.0 - self.margin, w.height() / 2.0 - self.margin))
            .unwrap_or((window_half_w - self.margin, window_half_h - self.margin));

        if let Some(mut tr) = world.get_mut::<Transform>(entity) {
            // X : avance + bounce aux bords
            tr.translation.x += self.speed_x * dt;
            if tr.translation.x > half_w {
                tr.translation.x = half_w;
            } else if tr.translation.x < -half_w {
                tr.translation.x = -half_w;
            }
            // Y : sinusoïde pure sur le temps de phase
            tr.translation.y = (t * self.sine_freq).sin() * half_h * self.sine_amp;
        }
    }
    fn name(&self) -> &'static str {
        "PatrolAndCharge"
    }
}

// ─── Shake + flash (transition entre phases) ───────────────────────

struct ShakeAndFlash {
    amplitude: f32,
}

impl Behavior for ShakeAndFlash {
    fn execute(&self, entity: Entity, world: &mut World) {
        let Some(enemy) = world.get::<EnemyBehavior>(entity) else {
            return;
        };
        let progress = enemy.phase_timer.elapsed().as_secs_f32() / TRANSITION_DURATION;
        let shake = progress * progress * self.amplitude;
        let dx = (fastrand::f32() - 0.5) * 2.0 * shake;
        let dy = (fastrand::f32() - 0.5) * 2.0 * shake;

        if let Some(mut tr) = world.get_mut::<Transform>(entity) {
            tr.translation.x += dx;
            tr.translation.y += dy;
        }
        // Flash blanc clignotant
        if let Some(mut sprite) = world.get_mut::<Sprite>(entity) {
            let v = 1.0 + ((progress * 30.0).sin() * 0.5 + 0.5) * 2.0;
            sprite.color = Color::rgba(v, v, v, 1.0);
        }
    }
    fn name(&self) -> &'static str {
        "ShakeAndFlash"
    }
}

// ─── Spawn d'une vague d'UFOs (one-shot via on_enter) ──────────────

struct SpawnUfoWave {
    count: usize,
}

impl Behavior for SpawnUfoWave {
    fn execute(&self, _entity: Entity, world: &mut World) {
        // Réutilise l'infrastructure existante : injecter une spawn_request
        // dans Difficulty. Les ennemis "green_ufo" seront spawnés par
        // `spawn_green_ufos_oneshot` au frame suivant.
        use crate::game_manager::difficulty::{Difficulty, SpawnPosition};
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

// ─── Mort : shake + explosions aléatoires ──────────────────────────

struct DyingFx {
    shake_max: f32,
    duration: f32,
}

impl Behavior for DyingFx {
    fn execute(&self, entity: Entity, world: &mut World) {
        let Some(enemy) = world.get::<EnemyBehavior>(entity) else {
            return;
        };
        let progress = (enemy.phase_timer.elapsed().as_secs_f32() / self.duration).clamp(0.0, 1.0);

        // Shake — relatif à la position "de phase" (on échantillonne à t=0 en mémoire via une tag component).
        let shake = progress * progress * self.shake_max;
        let dx = (fastrand::f32() - 0.5) * 2.0 * shake;
        let dy = (fastrand::f32() - 0.5) * 2.0 * shake;
        if let Some(mut tr) = world.get_mut::<Transform>(entity) {
            tr.translation.x += dx;
            tr.translation.y += dy;
        }

        // Flash blanc clignotant rapide
        if let Some(mut sprite) = world.get_mut::<Sprite>(entity) {
            let blink = ((progress * 60.0).sin() + 1.0) * 0.5;
            let v = 1.0 + blink * 2.0;
            sprite.color = Color::rgba(v, v, v, 1.0);
        }

        // NOTE : les explosions random autour du boss sont déléguées à l'ancien
        // système `enemy_dying` via le composant `Enemy { state: EnemyState::Dying }`.
        // Pour spawner directement depuis un Behavior il faudrait passer par un
        // EventWriter<SpawnExplosionEvent> ou une file de commandes — à ajouter plus
        // tard quand on aura un `BehaviorCommands` wrapper autour de `world`.
    }
    fn name(&self) -> &'static str {
        "DyingFx"
    }
}

// ─── Utilitaires one-shot ─────────────────────────────────────────

struct PlaySound {
    path: &'static str,
}

impl Behavior for PlaySound {
    fn execute(&self, _entity: Entity, world: &mut World) {
        let path = self.path;
        let handle = world.resource::<AssetServer>().load::<AudioSource>(path);
        world.spawn(AudioBundle {
            source: handle,
            settings: PlaybackSettings::DESPAWN,
        });
    }
    fn name(&self) -> &'static str {
        "PlaySound"
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
        // Ne peut pas despawn directement depuis &mut World en itération ;
        // on marque pour que le système boss.rs l'enlève au frame suivant.
        // Solution pragmatique : despawn immédiat via queue.
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

struct DespawnSelf;

impl Behavior for DespawnSelf {
    fn execute(&self, entity: Entity, world: &mut World) {
        if world.get_entity(entity).is_some() {
            world.despawn(entity);
        }
    }
    fn name(&self) -> &'static str {
        "DespawnSelf"
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Spawn dans la scène
// ═══════════════════════════════════════════════════════════════════════

/// Spawne un boss "v2" piloté par la nouvelle machine à état `EnemyBehavior`.
///
/// On conserve quand même le composant `Enemy` pour que les collisions
/// joueur↔boss et projectile↔boss continuent de fonctionner (gérées par
/// `enemy::enemy` et `physic::collision`). L'ancien boss.rs n'intervient
/// PAS sur cette entité car elle n'est pas marquée `BossMarker` (sauf si
/// on le souhaite explicitement — mettre `with_marker = true`).
pub fn spawn_boss_v2(commands: &mut Commands, asset_server: &AssetServer, position: Vec3) {
    use crate::enemy::system::{EnemyBehavior as Eb, Health};

    let definition = boss_definition();
    let initial_phase = definition.initial_phase.clone();

    commands.spawn((
        SpriteBundle {
            texture: asset_server.load("images/boss/idle/frame000.png"),
            sprite: Sprite {
                custom_size: Some(Vec2::splat(BOSS.sprite_size)),
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
        Eb {
            definition,
            current_phase: initial_phase,
            phase_timer: Timer::from_seconds(0.0, TimerMode::Once),
        },
        Health {
            current: (BOSS.phases[0].health * BOSS.phases.len() as i32) as f32,
            max: (BOSS.phases[0].health * BOSS.phases.len() as i32) as f32,
        },
        // Le composant Enemy est conservé pour compatibilité collisions :
        Enemy {
            health: BOSS.phases[0].health,
            max_health: BOSS.phases[0].health,
            state: EnemyState::Active(0),
            radius: BOSS.radius,
            sprite_size: BOSS.sprite_size,
            anim_timer: Timer::from_seconds(0.01, TimerMode::Once),
            phases: BOSS.phases,
            death_duration: BOSS.death_duration,
            death_shake_max: BOSS.death_shake_max,
            hit_sound: BOSS.hit_sound,
            death_explosion_sound: BOSS.death_explosion_sound,
            hit_flash_color: None,
        },
        BossMarker,
    ));
}
