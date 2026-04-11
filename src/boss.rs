//! Boss — ennemi spécifique utilisant le framework `enemy.rs`.
//!
//! Le boss est un `Enemy` avec :
//! - Intro en spirale (`Entering`) + animation de flexing (`Flexing`)
//! - 3 phases de combat avec mouvement patrol sinusoïdal
//! - Animation idle en boucle (cycle de frames)
//! - Musique dédiée lancée après l'intro
//! - Animation de mort avec flexing accéléré (par-dessus le dying générique)
//!
//! Les systèmes génériques (dégâts, flash, mort, projectiles) sont dans `EnemyPlugin`.

use crate::MusicMain;
use crate::asteroid::Asteroid;
use crate::difficulty::Difficulty;
use crate::enemies::BOSS;
use crate::enemy::{Enemy, EnemyState, PatrolMovement, PatternTimer};
use crate::explosion::load_frames_from_folder;
use crate::pause::not_paused;
use crate::player::Player;
use crate::state::GameState;
use bevy::prelude::*;

pub struct BossPlugin;

impl Plugin for BossPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, preload_boss_frames)
            .add_systems(
                Update,
                (
                    spawn_boss,
                    boss_intro,
                    boss_flexing,
                    boss_flexing_sound,
                    boss_music_delayed,
                    boss_idle_animation,
                    boss_dying_flexing,
                    boss_dying_stop_music,
                    boss_enable_patrol,
                    boss_charge_movement,
                    boss_pattern_executor,
                )
                    .run_if(in_state(GameState::Playing))
                    .run_if(not_paused),
            )
            .add_systems(
                Update,
                debug_skip_to_boss.run_if(in_state(GameState::Playing)),
            );
    }
}

// ─── Constantes boss ────────────────────────────────────────────────

const BOSS_SPAWN_TIME: f32 = 35.8;
const BOSS_START_ANIMATION_DURATION: f32 = 7.0;
const BOSS_MUSIC_DELAY: f32 = 1.0;
const BOSS_FLEXING_WAIT: f32 = 0.5;
const BOSS_START_2_ANIMATION_DURATION: f32 = 1.7;
const BOSS_TARGET_Y: f32 = 250.0;
const BOSS_INTRO_START_SCALE: f32 = 0.01;
const BOSS_INTRO_END_SCALE: f32 = 1.0;
const BOSS_SPIRAL_TURNS: f32 = 2.5;
const BOSS_SPIRAL_RADIUS: f32 = 150.0;
const BOSS_IDLE_FPS: f32 = 10.0;

// Patrol
const BOSS_MARGIN: f32 = 80.0;
const BOSS_PATROL_SPEED_X: f32 = 270.0;
const BOSS_SINE_AMPLITUDE_Y: f32 = 0.85;
const BOSS_SINE_FREQ_Y: f32 = 4.5;

// Charge
/// Vitesse de la charge vers le joueur (px/s).
const BOSS_CHARGE_SPEED: f32 = 2500.0;
/// Tolérance en Y pour déclencher la charge (px).
const BOSS_CHARGE_ALIGN_THRESHOLD: f32 = 50.0;
/// Vitesse de rotation pendant la charge (rad/s).
const BOSS_CHARGE_SPIN_SPEED: f32 = 15.0;

// ─── Composants spécifiques au boss ─────────────────────────────────

/// Marqueur pour identifier le boss parmi les Enemy.
#[derive(Component)]
pub struct BossMarker;

/// Animation idle du boss (cycle de frames).
#[derive(Component)]
struct BossIdleAnim {
    timer: Timer,
    current_frame: usize,
}

/// Marqueur pour la musique du boss (pause/cleanup).
#[derive(Component)]
pub struct MusicBoss;

/// Marqueur : le son de flexing a été joué.
#[derive(Component)]
struct BossFlexingSoundPlayed;

/// Le boss est en train de charger vers le joueur.
/// La direction est fixée au moment du lancement.
#[derive(Component)]
pub struct BossCharge {
    pub direction: Vec2,
}

/// Index du pattern actuel dans la liste de patterns de la phase.
#[derive(Component)]
pub struct BossPatternIndex(pub usize);

// ─── Ressources ─────────────────────────────────────────────────────

#[derive(Resource)]
struct BossIdleFrames(Vec<Handle<Image>>);

#[derive(Resource)]
struct BossFlexingFrames(Vec<Handle<Image>>);

// ─── Préchargement ──────────────────────────────────────────────────

fn preload_boss_frames(mut commands: Commands, asset_server: Res<AssetServer>) {
    let idle_frames = load_frames_from_folder(&asset_server, "images/boss/idle")
        .expect("boss idle frames folder missing or empty");
    commands.insert_resource(BossIdleFrames(idle_frames));

    let flexing_frames = load_frames_from_folder(&asset_server, "images/boss/flexing")
        .expect("boss flexing frames folder missing or empty");
    commands.insert_resource(BossFlexingFrames(flexing_frames));
}

// ─── Spawn ──────────────────────────────────────────────────────────

fn spawn_boss(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut difficulty: ResMut<Difficulty>,
    enemy_q: Query<&Enemy, With<BossMarker>>,
    windows: Query<&Window>,
) {
    if difficulty.elapsed < BOSS_SPAWN_TIME || !enemy_q.is_empty() || difficulty.boss_spawned {
        return;
    }
    difficulty.boss_spawned = true;

    let _window = windows.single();
    let start_y = 50.0;

    commands.spawn(AudioBundle {
        source: asset_server.load("audio/boss_start.ogg"),
        settings: PlaybackSettings::DESPAWN,
    });

    commands.spawn((
        SpriteBundle {
            texture: asset_server.load("images/boss/idle/frame000.png"),
            sprite: Sprite {
                custom_size: Some(Vec2::splat(BOSS.sprite_size)),
                color: Color::WHITE,
                ..default()
            },
            transform: Transform {
                translation: Vec3::new(0.0, start_y, 0.5),
                scale: Vec3::splat(BOSS_INTRO_START_SCALE),
                ..default()
            },
            ..default()
        },
        Enemy {
            health: BOSS.phases[0].health,
            max_health: BOSS.phases[0].health,
            state: EnemyState::Entering,
            radius: BOSS.radius,
            sprite_size: BOSS.sprite_size,
            anim_timer: Timer::from_seconds(BOSS_START_ANIMATION_DURATION, TimerMode::Once),
            phases: BOSS.phases,
            death_duration: BOSS.death_duration,
            death_shake_max: BOSS.death_shake_max,
            hit_sound: BOSS.hit_sound,
            death_explosion_sound: BOSS.death_explosion_sound,
        },
        BossMarker,
        BossIdleAnim {
            timer: Timer::from_seconds(1.0 / BOSS_IDLE_FPS, TimerMode::Repeating),
            current_frame: 0,
        },
        PatternTimer(Timer::from_seconds(
            BOSS.phases[0]
                .patterns
                .first()
                .map(|p| p.duration)
                .unwrap_or(1.0),
            TimerMode::Once,
        )),
        BossPatternIndex(0),
        PatrolMovement {
            dir_x: 1.0,
            sine_time: 0.0,
            initialized: false,
            enabled: false,
            speed_x: BOSS_PATROL_SPEED_X,
            sine_amplitude_y: BOSS_SINE_AMPLITUDE_Y,
            sine_freq_y: BOSS_SINE_FREQ_Y,
            margin: BOSS_MARGIN,
        },
    ));
}

// ─── Intro : spirale ────────────────────────────────────────────────

fn boss_intro(
    time: Res<Time>,
    mut boss_q: Query<(&mut Enemy, &mut Transform), (With<BossMarker>, Without<Player>)>,
) {
    for (mut enemy, mut transform) in boss_q.iter_mut() {
        if enemy.state != EnemyState::Entering {
            continue;
        }

        enemy.anim_timer.tick(time.delta());
        let progress = enemy.anim_timer.fraction();
        let eased = progress * progress * (3.0 - 2.0 * progress);

        let start_y = 50.0;
        let base_y = start_y + (BOSS_TARGET_Y - start_y) * eased;

        let spiral_progress = 1.0 - eased;
        let angle = progress * BOSS_SPIRAL_TURNS * std::f32::consts::TAU;
        let spiral_r = BOSS_SPIRAL_RADIUS * spiral_progress;
        let offset_x = angle.cos() * spiral_r;
        let offset_y = angle.sin() * spiral_r;

        transform.translation.x = offset_x;
        transform.translation.y = base_y + offset_y;

        let scale =
            BOSS_INTRO_START_SCALE + (BOSS_INTRO_END_SCALE - BOSS_INTRO_START_SCALE) * eased;
        transform.scale = Vec3::splat(scale);

        if enemy.anim_timer.finished() {
            enemy.state = EnemyState::Flexing;
            enemy.anim_timer = Timer::from_seconds(
                BOSS_FLEXING_WAIT + BOSS_START_2_ANIMATION_DURATION,
                TimerMode::Once,
            );
            transform.scale = Vec3::splat(BOSS_INTRO_END_SCALE);
            transform.translation.x = 0.0;
            transform.translation.y = BOSS_TARGET_Y;
        }
    }
}

// ─── Flexing ────────────────────────────────────────────────────────

fn boss_flexing(
    time: Res<Time>,
    flexing_frames: Res<BossFlexingFrames>,
    _idle_frames: Res<BossIdleFrames>,
    mut boss_q: Query<(Entity, &mut Enemy, &mut Handle<Image>), With<BossMarker>>,
    mut difficulty: ResMut<Difficulty>,
) {
    for (_entity, mut enemy, mut texture) in boss_q.iter_mut() {
        if enemy.state != EnemyState::Flexing {
            continue;
        }

        enemy.anim_timer.tick(time.delta());

        if enemy.anim_timer.finished() {
            // Flexing terminé, en attente de la musique + patrol.
            // L'idle animation prend le relais pour la texture.
            if difficulty.boss_active_time.is_none() {
                difficulty.boss_active_time = Some(difficulty.elapsed);
            }
            continue;
        }

        let elapsed = enemy.anim_timer.elapsed_secs();

        if elapsed < BOSS_FLEXING_WAIT {
            continue;
        }

        let flexing_elapsed = elapsed - BOSS_FLEXING_WAIT;
        let flexing_progress = (flexing_elapsed / BOSS_START_2_ANIMATION_DURATION).clamp(0.0, 1.0);
        let frame_count = flexing_frames.0.len();
        let frame_index = ((flexing_progress * frame_count as f32) as usize).min(frame_count - 1);
        *texture = flexing_frames.0[frame_index].clone();
    }
}

/// Joue boss_start_2.ogg une seule fois au début du flexing.
fn boss_flexing_sound(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut boss_q: Query<(Entity, &Enemy, Option<&BossFlexingSoundPlayed>), With<BossMarker>>,
) {
    for (entity, enemy, sound_played) in boss_q.iter_mut() {
        if enemy.state != EnemyState::Flexing {
            continue;
        }
        if enemy.anim_timer.elapsed_secs() < BOSS_FLEXING_WAIT {
            continue;
        }
        if sound_played.is_some() {
            continue;
        }
        commands.entity(entity).insert(BossFlexingSoundPlayed);
        commands.spawn(AudioBundle {
            source: asset_server.load("audio/boss_start_2.ogg"),
            settings: PlaybackSettings::DESPAWN,
        });
    }
}

// ─── Musique boss ───────────────────────────────────────────────────

fn boss_music_delayed(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut difficulty: ResMut<Difficulty>,
) {
    if difficulty.boss_music_played {
        return;
    }
    let Some(active_time) = difficulty.boss_active_time else {
        return;
    };
    if difficulty.elapsed - active_time >= BOSS_MUSIC_DELAY {
        difficulty.boss_music_played = true;
        difficulty.boss_music_start_time = Some(difficulty.elapsed);
        commands.spawn((
            AudioBundle {
                source: asset_server.load("audio/boss.ogg"),
                settings: PlaybackSettings::LOOP,
            },
            MusicBoss,
        ));
    }
}

// ─── Animation idle ─────────────────────────────────────────────────

fn boss_idle_animation(
    time: Res<Time>,
    frames: Res<BossIdleFrames>,
    mut boss_q: Query<(&Enemy, &mut BossIdleAnim, &mut Handle<Image>), With<BossMarker>>,
) {
    for (enemy, mut anim, mut texture) in boss_q.iter_mut() {
        match &enemy.state {
            EnemyState::Entering | EnemyState::Active(_) => {}
            EnemyState::Flexing => {
                // Pendant le flexing visuel, l'idle ne tourne que pendant l'attente initiale
                // Une fois le timer fini (attente musique+patrol), l'idle reprend
                let elapsed = enemy.anim_timer.elapsed_secs();
                if elapsed >= BOSS_FLEXING_WAIT && !enemy.anim_timer.finished() {
                    continue;
                }
            }
            _ => continue,
        }

        anim.timer.tick(time.delta());
        if anim.timer.just_finished() {
            anim.current_frame = (anim.current_frame + 1) % frames.0.len();
            *texture = frames.0[anim.current_frame].clone();
        }
    }
}

// ─── Flexing accéléré pendant la mort ───────────────────────────────

fn boss_dying_flexing(
    flexing_frames: Res<BossFlexingFrames>,
    mut boss_q: Query<(&Enemy, &mut Handle<Image>), With<BossMarker>>,
) {
    for (enemy, mut texture) in boss_q.iter_mut() {
        if enemy.state != EnemyState::Dying {
            continue;
        }

        let frame_count = flexing_frames.0.len();
        if frame_count == 0 {
            continue;
        }

        let progress = enemy.anim_timer.fraction();
        let elapsed = enemy.anim_timer.elapsed_secs();
        let anim_speed = 1.0 + progress * 4.0;
        let anim_pos = (elapsed * anim_speed / BOSS_START_2_ANIMATION_DURATION).fract();
        let frame_index = ((anim_pos * frame_count as f32) as usize).min(frame_count - 1);
        *texture = flexing_frames.0[frame_index].clone();
    }
}


// ─── Couper la musique à la mort ────────────────────────────────────

fn boss_dying_stop_music(
    mut commands: Commands,
    boss_q: Query<&Enemy, With<BossMarker>>,
    music_q: Query<Entity, With<MusicBoss>>,
) {
    for enemy in boss_q.iter() {
        if enemy.state != EnemyState::Dying {
            continue;
        }
        for entity in music_q.iter() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

// ─── Activation du patrol ───────────────────────────────────────────

fn boss_enable_patrol(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    difficulty: Res<Difficulty>,
    mut query: Query<(Entity, &mut Enemy, &mut PatrolMovement), With<BossMarker>>,
) {
    let active = match difficulty.boss_music_start_time {
        Some(start) => difficulty.elapsed >= start + 3.0,
        None => false,
    };
    for (_entity, mut enemy, mut patrol) in query.iter_mut() {
        if active && enemy.state == EnemyState::Flexing && enemy.anim_timer.finished() {
            // Le flexing visuel est fini ET le délai post-musique est écoulé → Active
            enemy.state = EnemyState::Active(0);
            if let Some(sound) = BOSS.phases[0].enter_sound {
                commands.spawn(AudioBundle {
                    source: asset_server.load(sound),
                    settings: PlaybackSettings::DESPAWN,
                });
            }
        }
        if matches!(enemy.state, EnemyState::Active(_)) {
            patrol.enabled = active;
        }
    }
}

// ─── Exécution des patterns (squelette) ─────────────────────────────

fn boss_pattern_executor(
    time: Res<Time>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut boss_q: Query<
        (
            Entity,
            &Enemy,
            &mut PatternTimer,
            &mut BossPatternIndex,
            &Transform,
            Option<&BossCharge>,
            &mut PatrolMovement,
        ),
        With<BossMarker>,
    >,
    player_q: Query<&Transform, (With<Player>, Without<BossMarker>)>,
) {
    for (entity, enemy, mut pattern_timer, mut pat_idx, boss_transform, charge, mut patrol) in
        boss_q.iter_mut()
    {
        let phase_idx = match &enemy.state {
            EnemyState::Active(idx) => *idx,
            _ => continue,
        };

        // Pas de pattern tant que le patrol n'est pas activé
        if !patrol.enabled && charge.is_none() {
            continue;
        }

        // Pas de nouveau pattern pendant une charge
        if charge.is_some() {
            continue;
        }

        pattern_timer.0.tick(time.delta());
        if !pattern_timer.0.just_finished() {
            continue;
        }

        let phase = &enemy.phases[phase_idx];
        if phase.patterns.is_empty() {
            continue;
        }

        // Cycle à travers les patterns de la phase
        let current_idx = pat_idx.0 % phase.patterns.len();
        let pattern = &phase.patterns[current_idx];
        pat_idx.0 += 1;

        // Programmer le timer pour le prochain pattern
        let next_idx = pat_idx.0 % phase.patterns.len();
        let next_duration = phase.patterns[next_idx].duration;
        pattern_timer.0 = Timer::from_seconds(next_duration, TimerMode::Once);

        match pattern.name {
            "charge" => {
                let Ok(player_transform) = player_q.get_single() else {
                    continue;
                };
                let boss_y = boss_transform.translation.y;
                let player_y = player_transform.translation.y;

                // Ne charge que si aligné en Y avec le joueur (tolérance)
                if (boss_y - player_y).abs() > BOSS_CHARGE_ALIGN_THRESHOLD {
                    // Pas aligné → on ne consomme pas le pattern, on réessaie vite
                    pat_idx.0 -= 1;
                    pattern_timer.0 = Timer::from_seconds(0.1, TimerMode::Once);
                    continue;
                }

                // Charge uniquement sur l'axe X (vers le joueur)
                let dir_x = if player_transform.translation.x < boss_transform.translation.x {
                    -1.0
                } else {
                    1.0
                };
                // Désactiver le patrol immédiatement pour éviter la sinusoïde ce frame
                patrol.enabled = false;
                commands.entity(entity).insert(BossCharge {
                    direction: Vec2::new(dir_x, 0.0),
                });
                commands.spawn(AudioBundle {
                    source: asset_server.load("audio/t_go.wav"),
                    settings: PlaybackSettings::DESPAWN,
                });
            }
            _ => {}
        }
    }
}

// ─── Charge : fonce en ligne droite vers le joueur ──────────────────

fn boss_charge_movement(
    mut commands: Commands,
    time: Res<Time>,
    windows: Query<&Window>,
    mut boss_q: Query<
        (
            Entity,
            &Enemy,
            &mut Transform,
            &BossCharge,
            &mut PatrolMovement,
            &mut PatternTimer,
            &BossPatternIndex,
        ),
        With<BossMarker>,
    >,
) {
    let window = windows.single();
    let half_w = window.width() / 2.0;
    let half_h = window.height() / 2.0;

    for (entity, enemy, mut transform, charge, mut patrol, mut pattern_timer, pat_idx) in
        boss_q.iter_mut()
    {
        let phase_idx = match &enemy.state {
            EnemyState::Active(idx) => *idx,
            _ => continue,
        };

        // Désactiver le patrol pendant la charge
        patrol.enabled = false;

        // Avancer en ligne droite
        let dt = time.delta_seconds();
        transform.translation.x += charge.direction.x * BOSS_CHARGE_SPEED * dt;
        transform.translation.y += charge.direction.y * BOSS_CHARGE_SPEED * dt;

        // Tourner sur lui-même pendant la charge
        transform.rotate_z(BOSS_CHARGE_SPIN_SPEED * dt);

        // Si le boss atteint un bord → fin de la charge, passage au pattern suivant
        let margin = BOSS_MARGIN;
        let at_edge = transform.translation.x.abs() >= half_w - margin
            || transform.translation.y.abs() >= half_h - margin;

        if at_edge {
            // Clamper à l'intérieur
            transform.translation.x = transform
                .translation
                .x
                .clamp(-(half_w - margin), half_w - margin);
            transform.translation.y = transform
                .translation
                .y
                .clamp(-(half_h - margin), half_h - margin);

            // Remettre la rotation à zéro
            transform.rotation = Quat::IDENTITY;

            // Retirer la charge et réactiver le patrol
            commands.entity(entity).remove::<BossCharge>();
            patrol.enabled = true;
            patrol.initialized = false;

            // Reprogrammer le timer avec la durée du prochain pattern
            let phase = &enemy.phases[phase_idx];
            let next_idx = pat_idx.0 % phase.patterns.len();
            pattern_timer.0 =
                Timer::from_seconds(phase.patterns[next_idx].duration, TimerMode::Once);
        }
    }
}

// ─── F3 : skip direct au flexing du boss ────────────────────────────

fn debug_skip_to_boss(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    asset_server: Res<AssetServer>,
    mut difficulty: ResMut<Difficulty>,
    boss_q: Query<Entity, With<BossMarker>>,
    asteroid_q: Query<Entity, With<Asteroid>>,
    music_q: Query<Entity, With<MusicMain>>,
    boss_music_q: Query<Entity, With<MusicBoss>>,
) {
    if !keyboard.just_pressed(KeyCode::F3) {
        return;
    }

    for entity in asteroid_q.iter() {
        commands.entity(entity).despawn_recursive();
    }
    for entity in music_q.iter() {
        commands.entity(entity).despawn_recursive();
    }
    for entity in boss_music_q.iter() {
        commands.entity(entity).despawn_recursive();
    }

    difficulty.elapsed = BOSS_SPAWN_TIME + 0.1;
    difficulty.spawning_stopped = true;
    difficulty.charging_played = true;
    difficulty.boom_played = true;
    difficulty.boom_14_played = true;
    difficulty.boom_18_played = true;
    difficulty.boom_22_played = true;
    difficulty.boss_music_played = false;
    difficulty.boss_music_start_time = None;
    difficulty.boss_active_time = None;
    difficulty.landing_played = true;
    difficulty.boss_spawned = true;

    for entity in boss_q.iter() {
        commands.entity(entity).despawn_recursive();
    }

    commands.spawn(AudioBundle {
        source: asset_server.load("audio/boss_start_2.ogg"),
        settings: PlaybackSettings::DESPAWN,
    });

    commands.spawn((
        SpriteBundle {
            texture: asset_server.load("images/boss/idle/frame000.png"),
            sprite: Sprite {
                custom_size: Some(Vec2::splat(BOSS.sprite_size)),
                color: Color::WHITE,
                ..default()
            },
            transform: Transform {
                translation: Vec3::new(0.0, BOSS_TARGET_Y, 0.5),
                scale: Vec3::splat(1.0),
                ..default()
            },
            ..default()
        },
        Enemy {
            health: BOSS.phases[0].health,
            max_health: BOSS.phases[0].health,
            state: EnemyState::Flexing,
            radius: BOSS.radius,
            sprite_size: BOSS.sprite_size,
            anim_timer: Timer::from_seconds(
                BOSS_FLEXING_WAIT + BOSS_START_2_ANIMATION_DURATION,
                TimerMode::Once,
            ),
            phases: BOSS.phases,
            death_duration: BOSS.death_duration,
            death_shake_max: BOSS.death_shake_max,
            hit_sound: BOSS.hit_sound,
            death_explosion_sound: BOSS.death_explosion_sound,
        },
        BossMarker,
        BossIdleAnim {
            timer: Timer::from_seconds(1.0 / BOSS_IDLE_FPS, TimerMode::Repeating),
            current_frame: 0,
        },
        PatternTimer(Timer::from_seconds(
            BOSS.phases[0]
                .patterns
                .first()
                .map(|p| p.duration)
                .unwrap_or(1.0),
            TimerMode::Once,
        )),
        BossPatternIndex(0),
        PatrolMovement {
            dir_x: 1.0,
            sine_time: 0.0,
            initialized: false,
            enabled: false,
            speed_x: BOSS_PATROL_SPEED_X,
            sine_amplitude_y: BOSS_SINE_AMPLITUDE_Y,
            sine_freq_y: BOSS_SINE_FREQ_Y,
            margin: BOSS_MARGIN,
        },
        BossFlexingSoundPlayed,
    ));
}
