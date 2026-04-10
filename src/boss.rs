//! Système de boss.
//!
//! Architecture extensible pour un boss de shoot'em up avec :
//! - État machine : Waiting → Entering → Active(Phase) → Dying → Dead
//! - Phases data-driven via PhaseDef (comme WeaponDef)
//! - Animation idle (12 frames) en boucle
//! - Entrée en spirale depuis la planète, synchronisée avec boss_start.ogg (~3.7s)
//! - Musique boss lancée à la fin de l'intro
//!
//! ## Ajouter un pattern
//! 1. Écrire `fn fire_pattern_xxx(commands, asset_server, origin, player_pos)`
//! 2. L'ajouter dans `PhaseDef.patterns`
//! 3. Ajouter le match dans `boss_pattern_executor`
//!
//! ## Ajouter une phase
//! 1. Ajouter une variante à `BossPhaseId`
//! 2. Définir une `const PHASE_N: PhaseDef`
//! 3. Ajouter dans `phase_def()` et `boss_phase_logic`

use crate::asteroid::Asteroid;
use crate::difficulty::Difficulty;
use crate::explosion::load_frames_from_folder;
use crate::missile::{Missile, missile_hits_circle};
use crate::pause::PauseState;
use crate::player::Player;
use crate::state::GameState;
use crate::MusicMain;
use bevy::prelude::*;

pub struct BossPlugin;

impl Plugin for BossPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, preload_boss_idle_frames)
            .add_systems(
                Update,
                (
                    spawn_boss,
                    boss_intro,
                    boss_flexing,
                    boss_flexing_sound,
                    boss_music_delayed,
                    boss_idle_animation,
                    boss_phase_logic,
                    boss_sinusoidal_movement,
                    boss_pattern_executor,
                    missile_boss_collision,
                    move_boss_projectiles,
                    boss_hit_flash,
                    boss_dying,
                    cleanup_boss_projectiles_offscreen,
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

fn not_paused(pause: Res<PauseState>) -> bool {
    !pause.paused
}

// ─── Constantes ─────────────────────────────────────────────────────

/// Temps (difficulty.elapsed) d'apparition du boss.
const BOSS_SPAWN_TIME: f32 = 35.8;
/// Durée de la première animation d'entrée (dézoom/spirale, = durée de boss_start.ogg ≈ 7s).
const BOSS_START_ANIMATION_DURATION: f32 = 7.0;
/// Délai après le passage en Active avant de lancer la musique boss (secondes).
const BOSS_MUSIC_DELAY: f32 = 1.0;
/// Pause entre les deux animations d'entrée.
const BOSS_FLEXING_WAIT: f32 = 0.5;
/// Durée de la deuxième animation d'entrée (flexing).
const BOSS_START_2_ANIMATION_DURATION: f32 = 1.7;
/// Points de vie du boss.
const BOSS_MAX_HEALTH: i32 = 150;
/// Rayon de la hitbox du boss.
const BOSS_RADIUS: f32 = 80.0;
/// Position Y cible après l'intro.
const BOSS_TARGET_Y: f32 = 250.0;
/// Scale initiale (minuscule, sort de la planète).
const BOSS_INTRO_START_SCALE: f32 = 0.01;
/// Scale finale (taille normale).
const BOSS_INTRO_END_SCALE: f32 = 1.0;
/// Taille du sprite boss.
const BOSS_SPRITE_SIZE: f32 = 256.0;
/// Nombre de tours de spirale pendant l'entrée.
const BOSS_SPIRAL_TURNS: f32 = 2.5;
/// Rayon maximal de la spirale.
const BOSS_SPIRAL_RADIUS: f32 = 150.0;
/// Durée du flash blanc au hit.
const BOSS_HIT_FLASH_DURATION: f32 = 0.06;
/// FPS de l'animation idle.
const BOSS_IDLE_FPS: f32 = 10.0;

// ─── Patrol pattern (Phase 1) ──────────────────────────────────────

/// Marge du boss par rapport au bord de l'écran (px).
const BOSS_MARGIN: f32 = 80.0;
/// Vitesse horizontale constante du boss (px/s).
const BOSS_PATROL_SPEED_X: f32 = 250.0;
/// Amplitude verticale du mouvement sinusoïdal (fraction de la demi-hauteur écran).
const BOSS_SINE_AMPLITUDE_Y: f32 = 0.85;
/// Fréquence verticale (rad/s) — oscillation haut/bas.
const BOSS_SINE_FREQ_Y: f32 = 3.0;

// ─── État machine ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BossState {
    /// En attente d'apparition.
    Waiting,
    /// Arrive en spirale depuis la planète, invincible.
    Entering,
    /// Attente + animation flexing après l'arrivée, invincible.
    Flexing,
    /// Actif dans une phase, vulnérable.
    Active(BossPhaseId),
    /// Animation de mort en cours, invincible.
    Dying,
    /// Mort, sera despawné.
    Dead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BossPhaseId {
    Phase1,
    Phase2,
    Phase3,
}

// ─── Définitions des phases (data-driven) ───────────────────────────

pub struct PhaseDef {
    /// Seuil de vie pour entrer dans cette phase (% de max_health).
    pub health_threshold_pct: f32,
    /// Noms des patterns utilisés par cette phase (squelette : vide).
    pub patterns: &'static [&'static str],
    /// Son joué à l'entrée de la phase.
    pub enter_sound: Option<&'static str>,
    /// Intervalle entre les activations de pattern (secondes).
    pub pattern_interval: f32,
}

pub const PHASE_1: PhaseDef = PhaseDef {
    health_threshold_pct: 1.0,
    patterns: &[],
    enter_sound: Some("audio/boom.wav"),
    pattern_interval: 2.0,
};

pub const PHASE_2: PhaseDef = PhaseDef {
    health_threshold_pct: 0.66,
    patterns: &[],
    enter_sound: Some("audio/boom.wav"),
    pattern_interval: 1.5,
};

pub const PHASE_3: PhaseDef = PhaseDef {
    health_threshold_pct: 0.33,
    patterns: &[],
    enter_sound: Some("audio/boom.wav"),
    pattern_interval: 1.0,
};

fn phase_def(id: BossPhaseId) -> &'static PhaseDef {
    match id {
        BossPhaseId::Phase1 => &PHASE_1,
        BossPhaseId::Phase2 => &PHASE_2,
        BossPhaseId::Phase3 => &PHASE_3,
    }
}

// ─── Composants ─────────────────────────────────────────────────────

#[derive(Component)]
pub struct Boss {
    pub health: i32,
    pub max_health: i32,
    pub state: BossState,
    pub radius: f32,
    /// Timer pour l'animation d'entrée ou de mort.
    pub anim_timer: Timer,
}

/// Animation idle du boss (cycle de frames).
#[derive(Component)]
struct BossIdleAnim {
    timer: Timer,
    current_frame: usize,
}

/// Flash blanc au hit (comme les astéroïdes).
#[derive(Component)]
pub struct BossHitFlash(pub Timer);

/// Cadence des patterns du boss.
#[derive(Component)]
pub struct PatternTimer(pub Timer);

/// Marqueur pour la musique du boss (pause/cleanup).
#[derive(Component)]
pub struct MusicBoss;

/// Projectile tiré par le boss.
#[derive(Component)]
pub struct BossProjectile {
    pub velocity: Vec3,
    pub radius: f32,
}

/// Direction horizontale du patrol (+1.0 ou -1.0), flip aux bords.
#[derive(Component)]
struct BossPatrol {
    dir_x: f32,
    sine_time: f32,
}

/// Marqueur : le son de flexing a été joué.
#[derive(Component)]
struct BossFlexingSoundPlayed;

/// Ressource : frames idle préchargées.
#[derive(Resource)]
struct BossIdleFrames(Vec<Handle<Image>>);

/// Ressource : frames flexing préchargées.
#[derive(Resource)]
struct BossFlexingFrames(Vec<Handle<Image>>);

// ─── Préchargement des frames idle ──────────────────────────────────

fn preload_boss_idle_frames(mut commands: Commands, asset_server: Res<AssetServer>) {
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
    difficulty: Res<Difficulty>,
    boss_q: Query<&Boss>,
    windows: Query<&Window>,
) {
    if difficulty.elapsed < BOSS_SPAWN_TIME || !boss_q.is_empty() {
        return;
    }

    let window = windows.single();
    let half_h = window.height() / 2.0;

    // Position de départ : juste au-dessus du centre de l'écran
    let start_y = 50.0;

    // Son d'entrée (boss_start.ogg, synchronisé avec l'animation)
    commands.spawn(AudioBundle {
        source: asset_server.load("audio/boss_start.ogg"),
        settings: PlaybackSettings::DESPAWN,
    });

    commands.spawn((
        SpriteBundle {
            texture: asset_server.load("images/boss/idle/frame000.png"),
            sprite: Sprite {
                custom_size: Some(Vec2::splat(BOSS_SPRITE_SIZE)),
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
        Boss {
            health: BOSS_MAX_HEALTH,
            max_health: BOSS_MAX_HEALTH,
            state: BossState::Entering,
            radius: BOSS_RADIUS,
            anim_timer: Timer::from_seconds(BOSS_START_ANIMATION_DURATION, TimerMode::Once),
        },
        BossIdleAnim {
            timer: Timer::from_seconds(1.0 / BOSS_IDLE_FPS, TimerMode::Repeating),
            current_frame: 0,
        },
        PatternTimer(Timer::from_seconds(
            PHASE_1.pattern_interval,
            TimerMode::Repeating,
        )),
        BossPatrol {
            dir_x: 1.0,
            sine_time: 0.0,
        },
    ));
}

// ─── Intro : spirale depuis la planète ──────────────────────────────

fn boss_intro(time: Res<Time>, mut boss_q: Query<(&mut Boss, &mut Transform), Without<Player>>) {
    for (mut boss, mut transform) in boss_q.iter_mut() {
        if boss.state != BossState::Entering {
            continue;
        }

        boss.anim_timer.tick(time.delta());
        let progress = boss.anim_timer.fraction();

        // Ease-in-out pour un mouvement fluide
        let eased = progress * progress * (3.0 - 2.0 * progress);

        let start_y = 50.0;

        // Position de base : du centre vers la position cible
        let base_y = start_y + (BOSS_TARGET_Y - start_y) * eased;

        // Spirale : le rayon diminue à mesure que le boss approche sa position
        let spiral_progress = 1.0 - eased; // diminue de 1→0
        let angle = progress * BOSS_SPIRAL_TURNS * std::f32::consts::TAU;
        let spiral_r = BOSS_SPIRAL_RADIUS * spiral_progress;
        let offset_x = angle.cos() * spiral_r;
        let offset_y = angle.sin() * spiral_r;

        transform.translation.x = offset_x;
        transform.translation.y = base_y + offset_y;

        // Scale : minuscule → taille normale (l'effet d'arrivée est le zoom progressif)
        let scale =
            BOSS_INTRO_START_SCALE + (BOSS_INTRO_END_SCALE - BOSS_INTRO_START_SCALE) * eased;
        transform.scale = Vec3::splat(scale);

        // Fin de l'intro → passage en Flexing (attente + animation flexing)
        if boss.anim_timer.finished() {
            boss.state = BossState::Flexing;
            boss.anim_timer = Timer::from_seconds(
                BOSS_FLEXING_WAIT + BOSS_START_2_ANIMATION_DURATION,
                TimerMode::Once,
            );
            transform.scale = Vec3::splat(BOSS_INTRO_END_SCALE);
            transform.translation.x = 0.0;
            transform.translation.y = BOSS_TARGET_Y;
        }
    }
}

// ─── Flexing : attente 0.5s puis animation flexing ─────────────────

fn boss_flexing(
    time: Res<Time>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    flexing_frames: Res<BossFlexingFrames>,
    idle_frames: Res<BossIdleFrames>,
    mut boss_q: Query<(Entity, &mut Boss, &mut Handle<Image>)>,
    mut difficulty: ResMut<Difficulty>,
) {
    for (_entity, mut boss, mut texture) in boss_q.iter_mut() {
        if boss.state != BossState::Flexing {
            continue;
        }

        boss.anim_timer.tick(time.delta());
        let elapsed = boss.anim_timer.elapsed_secs();

        // Phase d'attente : l'idle continue (géré par boss_idle_animation)
        if elapsed < BOSS_FLEXING_WAIT {
            continue;
        }

        // Progression dans l'animation flexing
        let flexing_elapsed = elapsed - BOSS_FLEXING_WAIT;
        let flexing_progress = (flexing_elapsed / BOSS_START_2_ANIMATION_DURATION).clamp(0.0, 1.0);
        let frame_count = flexing_frames.0.len();
        let frame_index = ((flexing_progress * frame_count as f32) as usize).min(frame_count - 1);
        *texture = flexing_frames.0[frame_index].clone();

        // Fin du flexing → Active (la musique boss sera lancée 3.5s plus tard)
        if boss.anim_timer.finished() {
            boss.state = BossState::Active(BossPhaseId::Phase1);
            *texture = idle_frames.0[0].clone();
            difficulty.boss_active_time = Some(difficulty.elapsed);

            if let Some(sound) = PHASE_1.enter_sound {
                commands.spawn(AudioBundle {
                    source: asset_server.load(sound),
                    settings: PlaybackSettings::DESPAWN,
                });
            }
        }
    }
}

/// Système qui lance boss_start_2.ogg une seule fois au début du flexing.
fn boss_flexing_sound(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut boss_q: Query<(Entity, &Boss, Option<&BossFlexingSoundPlayed>)>,
) {
    for (entity, boss, sound_played) in boss_q.iter_mut() {
        if boss.state != BossState::Flexing {
            continue;
        }
        if boss.anim_timer.elapsed_secs() < BOSS_FLEXING_WAIT {
            continue;
        }
        if sound_played.is_some() {
            continue;
        }
        // Premier tick après l'attente → son + marqueur
        commands.entity(entity).insert(BossFlexingSoundPlayed);
        commands.spawn(AudioBundle {
            source: asset_server.load("audio/boss_start_2.ogg"),
            settings: PlaybackSettings::DESPAWN,
        });
    }
}

/// Lance la musique boss 3.5s après le passage en Active.
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

// ─── Animation idle (cycle de frames) ───────────────────────────────

fn boss_idle_animation(
    time: Res<Time>,
    frames: Res<BossIdleFrames>,
    mut boss_q: Query<(&Boss, &mut BossIdleAnim, &mut Handle<Image>)>,
) {
    for (boss, mut anim, mut texture) in boss_q.iter_mut() {
        // Animer uniquement en Entering, Active, ou Flexing (pendant l'attente)
        match &boss.state {
            BossState::Entering | BossState::Active(_) => {}
            BossState::Flexing => {
                // Pendant le flexing, l'idle ne tourne que pendant l'attente
                if boss.anim_timer.elapsed_secs() >= BOSS_FLEXING_WAIT {
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

// ─── Transitions de phase ───────────────────────────────────────────

fn boss_phase_logic(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut boss_q: Query<(&mut Boss, &mut PatternTimer)>,
) {
    for (mut boss, mut pattern_timer) in boss_q.iter_mut() {
        let current_phase = match &boss.state {
            BossState::Active(phase) => *phase,
            _ => continue,
        };

        // Boss mort → passage en Dying
        if boss.health <= 0 {
            boss.state = BossState::Dying;
            boss.anim_timer = Timer::from_seconds(1.5, TimerMode::Once);
            continue;
        }

        let health_pct = boss.health as f32 / boss.max_health as f32;

        // Vérification des seuils de transition
        let next_phase =
            if health_pct <= PHASE_3.health_threshold_pct && current_phase != BossPhaseId::Phase3 {
                Some(BossPhaseId::Phase3)
            } else if health_pct <= PHASE_2.health_threshold_pct
                && current_phase != BossPhaseId::Phase2
                && current_phase != BossPhaseId::Phase3
            {
                Some(BossPhaseId::Phase2)
            } else {
                None
            };

        if let Some(new_phase) = next_phase {
            let def = phase_def(new_phase);
            boss.state = BossState::Active(new_phase);
            pattern_timer.0 = Timer::from_seconds(def.pattern_interval, TimerMode::Repeating);

            if let Some(sound) = def.enter_sound {
                commands.spawn(AudioBundle {
                    source: asset_server.load(sound),
                    settings: PlaybackSettings::DESPAWN,
                });
            }
        }
    }
}

// ─── Mouvement sinusoïdal du boss ──────────────────────────────────

/// Déplace le boss en sinusoïdes sur tout le terrain.
/// Activé uniquement quand la rotation planète/background a commencé
/// (3s après le lancement de la musique boss).
fn boss_sinusoidal_movement(
    time: Res<Time>,
    difficulty: Res<Difficulty>,
    mut boss_q: Query<(&Boss, &mut Transform, &mut BossPatrol), Without<Player>>,
    windows: Query<&Window>,
) {
    // Attendre que la rotation planète soit active
    let active = match difficulty.boss_music_start_time {
        Some(start) => difficulty.elapsed >= start + 3.0,
        None => false,
    };
    if !active {
        return;
    }

    let dt = time.delta_seconds();
    let window = windows.single();
    let half_w = window.width() / 2.0 - BOSS_MARGIN;
    let half_h = window.height() / 2.0 - BOSS_MARGIN;

    for (boss, mut transform, mut patrol) in boss_q.iter_mut() {
        match &boss.state {
            BossState::Active(_) => {}
            _ => continue,
        }

        // X : vitesse constante, flip aux bords
        transform.translation.x += patrol.dir_x * BOSS_PATROL_SPEED_X * dt;
        if transform.translation.x > half_w {
            transform.translation.x = half_w;
            patrol.dir_x = -1.0;
        } else if transform.translation.x < -half_w {
            transform.translation.x = -half_w;
            patrol.dir_x = 1.0;
        }

        // Y : sinusoïde
        patrol.sine_time += dt;
        let y = (patrol.sine_time * BOSS_SINE_FREQ_Y).sin() * half_h * BOSS_SINE_AMPLITUDE_Y;
        transform.translation.y = y;
    }
}

// ─── Exécution des patterns (squelette) ─────────────────────────────

fn boss_pattern_executor(
    time: Res<Time>,
    mut boss_q: Query<(&Boss, &mut PatternTimer, &Transform)>,
    _commands: Commands,
    _asset_server: Res<AssetServer>,
    _player_q: Query<&Transform, (With<Player>, Without<Boss>)>,
) {
    for (boss, mut pattern_timer, _boss_transform) in boss_q.iter_mut() {
        let _phase = match &boss.state {
            BossState::Active(phase) => *phase,
            _ => continue,
        };

        pattern_timer.0.tick(time.delta());
        if !pattern_timer.0.just_finished() {
            continue;
        }

        // ── SQUELETTE : ajouter les patterns de tir ici ──
        // let player_pos = _player_q.single().translation;
        // match _phase {
        //     BossPhaseId::Phase1 => fire_pattern_spread(...),
        //     BossPhaseId::Phase2 => fire_pattern_spiral(...),
        //     BossPhaseId::Phase3 => fire_pattern_barrage(...),
        // }
    }
}

// ─── Collision missiles joueur → boss ───────────────────────────────

fn missile_boss_collision(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    missile_q: Query<(Entity, &Transform, &Missile)>,
    mut boss_q: Query<(Entity, &Transform, &mut Boss, &mut Sprite)>,
) {
    for (boss_entity, boss_transform, mut boss, mut _boss_sprite) in boss_q.iter_mut() {
        // Invincible pendant l'intro et la mort
        match &boss.state {
            BossState::Active(_) => {}
            _ => continue,
        }

        for (missile_entity, missile_transform, missile) in missile_q.iter() {
            let hit = missile_hits_circle(
                missile_transform.translation.truncate(),
                missile_transform.rotation,
                &missile.hitbox,
                boss_transform.translation.truncate(),
                boss.radius,
            );

            if hit {
                boss.health -= 1;
                commands.entity(missile_entity).despawn();

                // Flash blanc
                commands
                    .entity(boss_entity)
                    .insert(BossHitFlash(Timer::from_seconds(
                        BOSS_HIT_FLASH_DURATION,
                        TimerMode::Once,
                    )));

                // Son de hit
                commands.spawn(AudioBundle {
                    source: asset_server.load("audio/asteroid_hit.ogg"),
                    settings: PlaybackSettings::DESPAWN,
                });
            }
        }
    }
}

// ─── Déplacement des projectiles boss ───────────────────────────────

fn move_boss_projectiles(mut query: Query<(&mut Transform, &BossProjectile)>, time: Res<Time>) {
    for (mut transform, proj) in query.iter_mut() {
        transform.translation += proj.velocity * time.delta_seconds();
    }
}

// ─── Flash blanc au hit ─────────────────────────────────────────────

fn boss_hit_flash(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Sprite, &mut BossHitFlash), With<Boss>>,
) {
    for (entity, mut sprite, mut flash) in query.iter_mut() {
        flash.0.tick(time.delta());

        if flash.0.finished() {
            sprite.color = Color::WHITE;
            commands.entity(entity).remove::<BossHitFlash>();
        } else {
            sprite.color = Color::rgba(100.0, 100.0, 100.0, 1.0);
        }
    }
}

// ─── Animation de mort ──────────────────────────────────────────────

fn boss_dying(
    mut commands: Commands,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    mut boss_q: Query<(Entity, &mut Boss, &mut Sprite, &Transform)>,
) {
    for (entity, mut boss, mut sprite, transform) in boss_q.iter_mut() {
        if boss.state != BossState::Dying {
            continue;
        }

        boss.anim_timer.tick(time.delta());
        let progress = boss.anim_timer.fraction();

        // Fade-out progressif
        sprite.color.set_a(1.0 - progress);

        // Spawner des explosions aléatoires autour du boss pendant la mort
        if fastrand::f32() < 0.15 {
            let offset = Vec3::new(
                (fastrand::f32() - 0.5) * 200.0,
                (fastrand::f32() - 0.5) * 200.0,
                1.0,
            );
            crate::explosion::spawn_explosion(
                &mut commands,
                &asset_server,
                transform.translation + offset,
                Vec2::splat(64.0),
                0,
                Vec3::ZERO,
                Quat::IDENTITY,
            );
        }

        if boss.anim_timer.finished() {
            boss.state = BossState::Dead;
            commands.entity(entity).despawn_recursive();

            // Son de mort
            commands.spawn(AudioBundle {
                source: asset_server.load("audio/asteroid_die.ogg"),
                settings: PlaybackSettings::DESPAWN,
            });
        }
    }
}

// ─── Nettoyage des projectiles hors écran ───────────────────────────

fn cleanup_boss_projectiles_offscreen(
    mut commands: Commands,
    query: Query<(Entity, &Transform), With<BossProjectile>>,
) {
    for (entity, transform) in query.iter() {
        let p = transform.translation;
        if p.x.abs() > 1200.0 || p.y.abs() > 900.0 {
            commands.entity(entity).despawn();
        }
    }
}

// ─── F3 : skip direct au flexing du boss ────────────────────────────

fn debug_skip_to_boss(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    asset_server: Res<AssetServer>,
    mut difficulty: ResMut<Difficulty>,
    boss_q: Query<Entity, With<Boss>>,
    asteroid_q: Query<Entity, With<Asteroid>>,
    music_q: Query<Entity, With<MusicMain>>,
    boss_music_q: Query<Entity, With<MusicBoss>>,
) {
    if !keyboard.just_pressed(KeyCode::F3) {
        return;
    }

    // Despawn tous les astéroïdes
    for entity in asteroid_q.iter() {
        commands.entity(entity).despawn_recursive();
    }

    // Couper la musique principale
    for entity in music_q.iter() {
        commands.entity(entity).despawn_recursive();
    }

    // Couper la musique boss si déjà en cours
    for entity in boss_music_q.iter() {
        commands.entity(entity).despawn_recursive();
    }

    // Avancer le temps juste après le spawn du boss
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

    // Despawn le boss existant s'il y en a un
    for entity in boss_q.iter() {
        commands.entity(entity).despawn_recursive();
    }

    // Son de flexing
    commands.spawn(AudioBundle {
        source: asset_server.load("audio/boss_start_2.ogg"),
        settings: PlaybackSettings::DESPAWN,
    });

    // Spawn le boss directement en état Flexing, à sa taille finale
    commands.spawn((
        SpriteBundle {
            texture: asset_server.load("images/boss/idle/frame000.png"),
            sprite: Sprite {
                custom_size: Some(Vec2::splat(BOSS_SPRITE_SIZE)),
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
        Boss {
            health: BOSS_MAX_HEALTH,
            max_health: BOSS_MAX_HEALTH,
            state: BossState::Flexing,
            radius: BOSS_RADIUS,
            anim_timer: Timer::from_seconds(
                BOSS_FLEXING_WAIT + BOSS_START_2_ANIMATION_DURATION,
                TimerMode::Once,
            ),
        },
        BossIdleAnim {
            timer: Timer::from_seconds(1.0 / BOSS_IDLE_FPS, TimerMode::Repeating),
            current_frame: 0,
        },
        PatternTimer(Timer::from_seconds(
            PHASE_1.pattern_interval,
            TimerMode::Repeating,
        )),
        BossPatrol {
            dir_x: 1.0,
            sine_time: 0.0,
        },
    ));
}
