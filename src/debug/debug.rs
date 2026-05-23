//! Mode debug.
//!
//! Touches :
//! - **F1** : toggle overlay (FPS, hitboxes via `Hitbox` + `CollisionLayer`,
//!   timer, difficulté, timeline, mouse coords) ET rend le joueur invulnérable
//!   (via le composant `Invulnerable`, géré par `debug_player_invulnerability`).
//! - **F2** : pendant l'intro → skip l'intro. Pendant Playing → saute la
//!   timeline jusqu'à `planet_appear` (juste avant le boss).
//! - **F3 / F4** : pendant l'intro → skip l'intro.
//! - **F5** : tue le joueur instantanément (test GameOver).

use crate::MusicMain;
use crate::behavior::behavior::BehaviorComponent;
use crate::enemy::asteroid::Asteroid;
use crate::enemy::boss::BossMarker;
use crate::enemy::enemy::Enemy;
use crate::game_manager::difficulty::Difficulty;
use crate::game_manager::game::{IntroData, IntroSound, LevelPhase, OutroCountdownData, OutroData};
use crate::level::level::{LevelRunner, Trigger};
use crate::menu::pause::PauseState;
use crate::movement::bounding_radius::BoundingRadius;
use crate::movement::movement_zone::MovementZone;
use crate::physic::area_of_effect::AreaOfEffect;
use crate::physic::collider::{layers, CollisionLayer, Hitbox};
use crate::physic::health::Health;
use crate::physic::invulnerable::DebugInvulnerable;
use crate::physic::player_detection::PlayerDetection;
use crate::player::player::Player;
use crate::ui::score::Score;
use crate::weapon::projectile::Projectile;
use crate::geometry::shape::Shape;
use bevy::prelude::*;

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DebugMode(false))
            .insert_resource(DebugMousePos(Vec2::ZERO))
            .add_systems(Startup, setup_debug_ui)
            .add_systems(
                Update,
                (
                    debug_skip_intro,
                    toggle_debug,
                    debug_player_invulnerability,
                    draw_hitboxes,
                    update_debug_ui,
                    update_debug_level_ui,
                    manage_asteroid_labels,
                    debug_mouse_coords,
                    debug_kill_player,
                    draw_zone_labels,
                ),
            );
    }
}

#[derive(Component)]
struct AsteroidLabel(Entity);

/// Marker sur les `Text2d` de labels de zones débug. Recréés chaque frame
/// (despawn-tous-puis-respawn).
#[derive(Component)]
struct DebugZoneLabel;

#[derive(Resource)]
pub struct DebugMode(pub bool);

#[derive(Resource)]
struct DebugMousePos(Vec2);

#[derive(Component)]
struct DebugMouseUI;

#[derive(Component)]
struct DebugUI;

#[derive(Component)]
struct DebugLevelUI;

fn setup_debug_ui(mut commands: Commands) {
    // Panneau gauche : infos générales
    commands.spawn((
        Text::new(""),
        TextFont { font_size: 16.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        Visibility::Hidden,
        GlobalZIndex(100),
        DebugUI,
    ));

    // Coordonnées souris (en bas à gauche)
    commands.spawn((
        Text::new("Mouse: (0, 0)"),
        TextFont { font_size: 16.0, ..default() },
        TextColor(Color::srgba(0.0, 1.0, 1.0, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        Visibility::Hidden,
        GlobalZIndex(100),
        DebugMouseUI,
    ));

    // Panneau droit : timeline du niveau
    commands.spawn((
        Text::new(""),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            ..default()
        },
        Visibility::Hidden,
        GlobalZIndex(100),
        DebugLevelUI,
    ));
}

fn toggle_debug(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut debug: ResMut<DebugMode>,
    mut ui_q: Query<&mut Visibility, (With<DebugUI>, Without<DebugLevelUI>, Without<DebugMouseUI>)>,
    mut level_ui_q: Query<&mut Visibility, (With<DebugLevelUI>, Without<DebugUI>, Without<DebugMouseUI>)>,
    mut mouse_ui_q: Query<&mut Visibility, (With<DebugMouseUI>, Without<DebugUI>, Without<DebugLevelUI>)>,
    mut difficulty: ResMut<crate::game_manager::difficulty::Difficulty>,
    runner: Option<ResMut<crate::level::level::LevelRunner>>,
    music_q: Query<Entity, With<MusicMain>>,
    asteroid_q: Query<Entity, With<Asteroid>>,
    mut boom_events: MessageWriter<crate::game_manager::difficulty::BoomEvent>,
    mut countdown_events: MessageWriter<crate::ui::countdown::CountdownEvent>,
    asset_server: Res<AssetServer>,
    sfx_library: Res<crate::audio::SfxLibrary>,
) {
    if keyboard.just_pressed(KeyCode::F2) {
        // Nettoyer les entités en jeu
        for entity in asteroid_q.iter() {
            if let Ok(mut e) = commands.get_entity(entity) { e.try_despawn(); }
        }

        // Avancer le LevelRunner jusqu'à "planet_appear" (juste avant le boss)
        if let Some(mut runner) = runner {
            // Synchroniser difficulty.elapsed AVANT d'exécuter les actions
            // (StartBgDeceleration et ShowPlanet utilisent difficulty.elapsed)
            difficulty.elapsed = 28.0;

            let all_actions = runner.skip_to("planet_appear", 28.0);
            for actions in &all_actions {
                for action in actions {
                    // Ignorer les actions cosmétiques (sons, booms, countdown, musique)
                    if !action.should_replay_on_skip() {
                        continue;
                    }
                    crate::level::level::execute_action(
                        action,
                        &mut commands,
                        &asset_server,
                        &mut boom_events,
                        &mut countdown_events,
                        &mut difficulty,
                        &music_q,
                        &sfx_library,
                    );
                }
            }
        }
    }

    if keyboard.just_pressed(KeyCode::F1) {
        debug.0 = !debug.0;
        let new_vis = if debug.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if let Ok(mut vis) = ui_q.single_mut() {
            *vis = new_vis;
        }
        if let Ok(mut vis) = level_ui_q.single_mut() {
            *vis = new_vis;
        }
        if let Ok(mut vis) = mouse_ui_q.single_mut() {
            *vis = new_vis;
        }
    }
}

/// Insère/retire `DebugInvulnerable` sur le joueur selon l'état de
/// `DebugMode`. Marker distinct de `Invulnerable` pour ne PAS écraser un
/// `Invulnerable` posé par d'autres systèmes (shield, dash, etc.).
/// `apply_damage` filtre sur les deux markers.
pub fn debug_player_invulnerability(
    mut commands: Commands,
    debug: Res<DebugMode>,
    player_q: Query<(Entity, Option<&DebugInvulnerable>), With<Player>>,
) {
    let Ok((player_e, has_marker)) = player_q.single() else { return };
    match (debug.0, has_marker) {
        (true, None) => {
            commands.entity(player_e).insert(DebugInvulnerable);
        }
        (false, Some(_)) => {
            commands.entity(player_e).remove::<DebugInvulnerable>();
        }
        _ => {}
    }
}

fn update_debug_ui(
    debug: Res<DebugMode>,
    time: Res<Time>,
    difficulty: Res<Difficulty>,
    score: Res<Score>,
    mut ui_q: Query<&mut Text, With<DebugUI>>,
    player_q: Query<(&Transform, &Health), With<Player>>,
    enemy_q: Query<
        (
            &Enemy,
            &Health,
            &Transform,
            &BehaviorComponent,
        ),
        Without<Player>,
    >,
    asteroid_q: Query<&Asteroid>,
    projectile_q: Query<&Projectile>,
) {
    if !debug.0 {
        return;
    }

    let fps = 1.0 / time.delta_secs();
    let elapsed = difficulty.elapsed;
    let factor = difficulty.factor;

    let minutes = (elapsed / 60.0) as u32;
    let seconds = (elapsed % 60.0) as u32;

    let (player_pos, player_hp) = player_q
        .single()
        .map(|(t, h)| {
            (
                format!("({:.0}, {:.0})", t.translation.x, t.translation.y),
                h.current,
            )
        })
        .unwrap_or_else(|_| ("N/A".to_string(), 0));

    let mut enemy_lines = String::new();
    for (enemy, health, transform, behavior_comp) in enemy_q.iter() {
        let name = enemy.name;
        let pos = format!(
            "({:.0}, {:.0})",
            transform.translation.x, transform.translation.y
        );
        enemy_lines.push_str(&format!(
            "\n  {} {} | HP {}/{} ",
            name, pos, health.current, health.max
        ));
    }

    let asteroid_count = asteroid_q.iter().count();
    let missile_count = projectile_q.iter().count();

    if let Ok(mut text) = ui_q.single_mut() {
        **text = format!(
            "[DEBUG] GOD MODE\n\
             FPS        : {:.0}\n\
             Timer      : {:02}:{:02}\n\
             Difficulte : x{:.2}\n\
             Vies       : {}\n\
             Score      : {}\n\
             Player     : {}\n\
             Asteroides : {}\n\
             Missiles   : {}\n\
             \n\
             Enemy   : {}\n\
             F1 : Debug Mode ON/OFF\n\
             F2 : Skip asteroides\n\
             F3 : Skip au boss\n\
             F4 : Win niveau (outro)\n\
             F5 : Game Over (mort)",
            fps, minutes, seconds, factor,
            player_hp,
            score.value(),
            player_pos,
            asteroid_count,
            missile_count,
            if enemy_lines.is_empty() { "\n  (aucun)".to_string() } else { enemy_lines },
        );
    }
}

fn update_debug_level_ui(
    debug: Res<DebugMode>,
    runner: Option<Res<LevelRunner>>,
    progress: Res<crate::game_manager::game::GameProgress>,
    mut ui_q: Query<&mut Text, With<DebugLevelUI>>,
    level_phase: Option<Res<State<LevelPhase>>>,
    intro_data: Option<Res<IntroData>>,
    countdown_data: Option<Res<OutroCountdownData>>,
    outro_data: Option<Res<OutroData>>,
) {
    if !debug.0 {
        return;
    }

    let Some(runner) = runner else {
        return;
    };

    let steps = runner.steps();
    let current_idx = runner.current_index();
    let elapsed = runner.elapsed;

    let name = crate::level::level::level_name(progress.current_level);
    let mut lines = format!("--- {} (Niveau {}) ---\n", name, progress.current_level);

    // Phase courante : on lit l'enum discriminant via `State<LevelPhase>` et
    // les détails (timers, elapsed) via la Resource éphémère de chaque phase.
    if let Some(phase) = level_phase {
        let phase_str = match phase.get() {
            LevelPhase::Intro => match intro_data.as_deref() {
                Some(d) => format!(
                    "INTRO  {:.1}s / {:.1}s  son:{}",
                    d.elapsed,
                    d.duration,
                    if d.sound_finished { "fini" } else { "en cours" },
                ),
                None => "INTRO (skip editor)".to_string(),
            },
            LevelPhase::Running => "RUNNING".to_string(),
            LevelPhase::OutroCountdown => match countdown_data.as_deref() {
                Some(d) => {
                    let remaining = d.timer.duration().as_secs_f32() - d.timer.elapsed_secs();
                    format!("OUTRO COUNTDOWN  {:.1}s", remaining)
                }
                None => "OUTRO COUNTDOWN".to_string(),
            },
            LevelPhase::Outro => match outro_data.as_deref() {
                Some(d) => format!("OUTRO  {:.1}s", d.elapsed),
                None => "OUTRO".to_string(),
            },
        };
        lines.push_str(&format!("Phase : {}\n", phase_str));
    }
    lines.push('\n');

    for (i, step) in steps.iter().enumerate() {
        // ─── Indicateur de statut ───────────────────────────────
        let (status, status_detail) = if i < current_idx {
            // Étape exécutée
            let trigger_t = runner.trigger_time(step.label).unwrap_or(0.0);
            (
                "DONE",
                format!("  {:.1}s", trigger_t),
            )
        } else if i == current_idx {
            // Prochaine étape
            let eta = match &step.trigger {
                Trigger::AtTime(t) => {
                    let remaining = t - elapsed;
                    if remaining > 0.0 {
                        format!("  dans {:.1}s", remaining)
                    } else {
                        "  imminent".to_string()
                    }
                }
                Trigger::AfterPrevious(d) => {
                    // Le previous est la dernière étape exécutée
                    let prev_time = if current_idx > 0 {
                        runner.trigger_time(steps[current_idx - 1].label).unwrap_or(0.0)
                    } else {
                        0.0
                    };
                    let target = prev_time + d;
                    let remaining = target - elapsed;
                    if remaining > 0.0 {
                        format!("  dans {:.1}s", remaining)
                    } else {
                        "  imminent".to_string()
                    }
                }
                Trigger::After(label, d) => {
                    if let Some(ref_time) = runner.trigger_time(label) {
                        let target = ref_time + d;
                        let remaining = target - elapsed;
                        if remaining > 0.0 {
                            format!("  dans {:.1}s", remaining)
                        } else {
                            "  imminent".to_string()
                        }
                    } else {
                        format!("  attend '{}'", label)
                    }
                }
            };
            ("NEXT", eta)
        } else {
            // Étape future
            let eta = match &step.trigger {
                Trigger::AtTime(t) => {
                    let remaining = t - elapsed;
                    format!("  dans {:.0}s", remaining)
                }
                _ => String::new(),
            };
            ("....", eta)
        };

        // ─── Trigger description ────────────────────────────────
        let trigger_desc = step.trigger.short_desc();

        // ─── Actions courtes ────────────────────────────────────
        let actions_str: Vec<String> = step.actions.iter().map(|a| a.short_name()).collect();
        let actions_joined = actions_str.join(", ");

        // ─── Lien de causalité ──────────────────────────────────
        let chain_info = match &step.trigger {
            Trigger::After(label, delay) => {
                let ref_status = if runner.trigger_time(label).is_some() {
                    "DONE"
                } else if steps.iter().any(|s| s.label == *label) {
                    "WAIT"
                } else {
                    "???"
                };
                format!("\n          chaine : {} +{:.1}s [{}]", label, delay, ref_status)
            }
            _ => String::new(),
        };

        lines.push_str(&format!(
            "  {}  {:<16} {}{}  {}\n{}\n",
            status, step.label, trigger_desc, status_detail, actions_joined, chain_info,
        ));
    }

    // ─── Résumé de progression ──────────────────────────────────
    lines.push_str(&format!(
        "\nProgression : {}/{} etapes  |  {:.1}s\n",
        current_idx,
        steps.len(),
        elapsed,
    ));

    if runner.is_finished() {
        lines.push_str("Toutes les etapes executees\n");
        lines.push_str("(en attente de MarkLevelComplete)\n");
    }

    if let Ok(mut text) = ui_q.single_mut() {
        **text = lines;
    }
}

fn manage_asteroid_labels(
    mut commands: Commands,
    debug: Res<DebugMode>,
    asteroid_q: Query<(Entity, &Transform, &Asteroid)>,
    mut label_q: Query<
        (Entity, &AsteroidLabel, &mut Transform, &mut Visibility),
        Without<Asteroid>,
    >,
) {
    for (label_entity, label, _, _) in label_q.iter() {
        if asteroid_q.get(label.0).is_err() {
            if let Ok(mut e) = commands.get_entity(label_entity) { e.try_despawn(); }
        }
    }

    if !debug.0 {
        for (_, _, _, mut vis) in label_q.iter_mut() {
            *vis = Visibility::Hidden;
        }
        return;
    }

    let mut labeled: std::collections::HashSet<Entity> = std::collections::HashSet::new();
    for (_, label, mut label_transform, mut vis) in label_q.iter_mut() {
        labeled.insert(label.0);
        if let Ok((_, asteroid_transform, asteroid)) = asteroid_q.get(label.0) {
            label_transform.translation = Vec3::new(
                asteroid_transform.translation.x,
                asteroid_transform.translation.y + asteroid.radius + 15.0,
                10.0,
            );
            *vis = Visibility::Visible;
        }
    }

    for (entity, transform, asteroid) in asteroid_q.iter() {
        if labeled.contains(&entity) {
            continue;
        }
        //let name = format!("x{:03}", asteroid.texture_index);
        commands.spawn((
            AsteroidLabel(entity),
        ));
    }
}

/// Dessine la zone de détection joueur via gizmos. Couleur : magenta normalement,
/// rouge si le joueur est dedans, gris si en cooldown.
fn draw_player_detection(
    gizmos: &mut Gizmos,
    query: &Query<(&Transform, &PlayerDetection)>,
) {
    for (transform, detection) in query.iter() {
        let pos = transform.translation.truncate();
        let color = if detection.inside {
            Color::srgb(1.0, 0.2, 0.2)
        } else if detection.cooldown_remaining > 0.0 {
            Color::srgb(0.5, 0.5, 0.5)
        } else {
            Color::srgb(1.0, 0.0, 1.0)
        };
        match &detection.shape {
            Shape::Circle(r) => {
                gizmos.circle_2d(pos, *r, color);
            }
            Shape::Rect {
                half_length,
                half_width,
            } => {
                let angle = transform.rotation.to_euler(EulerRot::ZYX).0;
                let cos = angle.cos();
                let sin = angle.sin();
                let ax = Vec2::new(cos, sin);
                let ay = Vec2::new(-sin, cos);
                let corners = [
                    pos + ax * *half_width + ay * *half_length,
                    pos - ax * *half_width + ay * *half_length,
                    pos - ax * *half_width - ay * *half_length,
                    pos + ax * *half_width - ay * *half_length,
                ];
                for i in 0..4 {
                    gizmos.line_2d(corners[i], corners[(i + 1) % 4], color);
                }
            }
        }
    }
}

/// Couleur du gizmo selon la `CollisionLayer` de l'entité.
fn color_for_layer(layer: u32) -> Color {
    if layer & layers::PLAYER != 0 {
        Color::srgb(0.0, 1.0, 0.0)
    } else if layer & layers::ASTEROID != 0 {
        Color::srgb(1.0, 0.0, 0.0)
    } else if layer & layers::ENEMY != 0 {
        Color::srgb(0.0, 1.0, 1.0)
    } else if layer & layers::PLAYER_PROJECTILE != 0 {
        Color::srgb(1.0, 1.0, 0.0)
    } else if layer & layers::ENEMY_PROJECTILE != 0 {
        Color::srgb(1.0, 0.5, 0.0)
    } else if layer & layers::AOE != 0 {
        Color::srgb(1.0, 0.0, 1.0)
    } else if layer & layers::ITEM != 0 {
        Color::srgb(0.0, 0.5, 1.0)
    } else {
        Color::WHITE
    }
}

/// Dessine la `Hitbox` de toutes les entités collidables (Hitbox + CollisionLayer),
/// avec une couleur par layer.
fn draw_colliders(
    gizmos: &mut Gizmos,
    query: &Query<(&Transform, &Hitbox, &CollisionLayer)>,
) {
    for (transform, hitbox, layer) in query.iter() {
        let pos = transform.translation.truncate();
        let color = color_for_layer(layer.0);
        match &hitbox.0 {
            Shape::Circle(r) => {
                gizmos.circle_2d(pos, *r, color);
            }
            Shape::Rect {
                half_length,
                half_width,
            } => {
                let angle = transform.rotation.to_euler(EulerRot::ZYX).0;
                let cos = angle.cos();
                let sin = angle.sin();
                let ax = Vec2::new(cos, sin);
                let ay = Vec2::new(-sin, cos);
                let corners = [
                    pos + ax * half_width + ay * half_length,
                    pos - ax * half_width + ay * half_length,
                    pos - ax * half_width - ay * half_length,
                    pos + ax * half_width - ay * half_length,
                ];
                for i in 0..4 {
                    gizmos.line_2d(corners[i], corners[(i + 1) % 4], color);
                }
            }
        }
    }
}

fn debug_mouse_coords(
    debug: Res<DebugMode>,
    window: Single<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut mouse_pos: ResMut<DebugMousePos>,
    mut mouse_ui_q: Query<&mut Text, With<DebugMouseUI>>,
) {
    if !debug.0 {
        return;
    }
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    // Convertir en coordonnées world
    let world_pos = if let Ok((camera, cam_transform)) = camera_q.single() {
        camera
            .viewport_to_world_2d(cam_transform, cursor_pos)
            .unwrap_or(Vec2::ZERO)
    } else {
        Vec2::ZERO
    };

    mouse_pos.0 = world_pos;

    // Mettre à jour l'UI
    if let Ok(mut text) = mouse_ui_q.single_mut() {
        **text = format!(
            "Mouse: ({:.0}, {:.0})  |  Screen: ({:.0}, {:.0})",
            world_pos.x, world_pos.y, cursor_pos.x, cursor_pos.y,
        );
    }

    // Clic droit → log dans la console
    if mouse.just_pressed(MouseButton::Right) {
        info!(
            ">>> CLICK  world=({:.1}, {:.1})  screen=({:.1}, {:.1})",
            world_pos.x, world_pos.y, cursor_pos.x, cursor_pos.y,
        );
    }
}

/// F2/F3 pendant l'intro : skip l'intro et passe en Running.
/// (F4 a son propre handler dans `game.rs` qui saute directement à l'outro.)
/// Doit tourner avant toggle_debug et debug_skip_to_boss pour que
/// l'intro soit déjà terminée quand ces systèmes s'exécutent.
fn debug_skip_intro(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    intro_data: Option<Res<IntroData>>,
    mut player_q: Query<&mut Transform, With<Player>>,
    intro_sound_q: Query<Entity, With<IntroSound>>,
    windows: Query<&Window>,
    config: Res<crate::level::level::LevelConfig>,
    mut next: ResMut<NextState<LevelPhase>>,
) {
    if !keyboard.just_pressed(KeyCode::F2) && !keyboard.just_pressed(KeyCode::F3) {
        return;
    }
    let Some(data) = intro_data else { return };
    crate::game_manager::game::do_skip_intro(
        &mut commands,
        &data,
        &mut next,
        &mut player_q,
        &intro_sound_q,
        &windows,
        &config,
    );
}

fn debug_kill_player(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    state: Res<State<crate::game_manager::state::GameState>>,
    mut next_state: ResMut<NextState<crate::game_manager::state::GameState>>,
    mut player_q: Query<(Entity, &mut Health), With<Player>>,
) {
    if keyboard.just_pressed(KeyCode::F5) && *state.get() == crate::game_manager::state::GameState::Playing {
        for (entity, mut health) in player_q.iter_mut() {
            health.current = 0;
            if let Ok(mut e) = commands.get_entity(entity) { e.try_despawn(); }
        }
        next_state.set(crate::game_manager::state::GameState::GameOver);
    }
}

fn draw_hitboxes(
    debug: Res<DebugMode>,
    mut gizmos: Gizmos,
    collider_q: Query<(&Transform, &Hitbox, &CollisionLayer)>,
    detection_q: Query<(&Transform, &PlayerDetection)>,
    zone_q: Query<(
        &crate::movement::movement_zone::MovementZone,
        Option<&crate::movement::bounding_radius::BoundingRadius>,
    )>,
    sprite_q: Query<(&Transform, &Sprite), Without<AreaOfEffect>>,
    window: Single<&Window>,
    camera_q: Query<&Projection>,
) {
    if !debug.0 {
        return;
    }

    draw_colliders(&mut gizmos, &collider_q);
    draw_player_detection(&mut gizmos, &detection_q);

    // Boîtes blanches semi-transparentes : taille effective des sprites
    // (Sprite.custom_size). Utile pour comparer la taille rendue avec le
    // BoundingRadius et la MovementZone. Skip les AOE — leur sprite est un
    // PNG carré à fond transparent qui dépasse de la vraie zone circulaire ;
    // afficher le carré induit en erreur. Le collider magenta suffit.
    for (transform, sprite) in sprite_q.iter() {
        if let Some(size) = sprite.custom_size {
            gizmos.rect_2d(
                Isometry2d::from_translation(transform.translation.truncate()),
                size,
                Color::srgba(1.0, 1.0, 1.0, 0.6),
            );
        }
    }

    // MovementZones : magenta = zone brute (centre clampé), rose = zone effective
    // (rétrécie par BoundingRadius, là où le bord du sprite vient s'arrêter).
    let w = window.physical_width() as f32;
    let h = window.physical_height() as f32;

    // Rectangle de référence vert : ce que la caméra voit réellement (projection.area).
    // À comparer avec le magenta : s'ils ne coïncident pas, il y a un décalage
    // entre window.width() et la taille rendue (DPI / scale factor).
    for projection in camera_q.iter() {
        if let Projection::Orthographic(ortho) = projection {
            let area = ortho.area;
            let cam_center = Vec2::new(
                (area.min.x + area.max.x) * 0.5,
                (area.min.y + area.max.y) * 0.5,
            );
            let cam_size = Vec2::new(area.max.x - area.min.x, area.max.y - area.min.y);
            gizmos.rect_2d(
                Isometry2d::from_translation(cam_center),
                cam_size,
                Color::srgb(0.0, 1.0, 0.0),
            );
        }
    }
    for (zone, bounding) in zone_q.iter() {
        let min_x_raw = (zone.margin.x - 0.5) * w;
        let max_x_raw = (0.5 - zone.margin.x) * w;
        let min_y_raw = (zone.margin.y - 0.5) * h;
        let max_y_raw = (0.5 - zone.margin.y) * h;
        let size_raw = Vec2::new(max_x_raw - min_x_raw, max_y_raw - min_y_raw);
        let center = Vec2::new((min_x_raw + max_x_raw) * 0.5, (min_y_raw + max_y_raw) * 0.5);
        gizmos.rect_2d(
            Isometry2d::from_translation(center),
            size_raw,
            Color::srgb(1.0, 0.0, 1.0),
        );

        if let Some(b) = bounding {
            let r = b.0;
            let size_eff = Vec2::new(size_raw.x - 2.0 * r, size_raw.y - 2.0 * r);
            if size_eff.x > 0.0 && size_eff.y > 0.0 {
                gizmos.rect_2d(
                    Isometry2d::from_translation(center),
                    size_eff,
                    Color::srgb(1.0, 0.5, 0.8),
                );
            }
        }
    }
    let _ = (w, h);
}

// Dessin debug des tourelles/mothership retiré avec la suppression des
// modules correspondants. À réimplémenter quand gatling/mothership seront
// réécrits.

/// Étiquette texte affichée pour un layer de collision. Renvoie `None` pour
/// les types trop nombreux à labeliser (projectiles).
fn collider_label(layer: u32) -> Option<&'static str> {
    if layer & layers::PLAYER != 0 {
        Some("PLAYER")
    } else if layer & layers::AOE != 0 {
        Some("AOE")
    } else if layer & layers::ASTEROID != 0 {
        Some("ASTEROID")
    } else if layer & layers::ENEMY != 0 {
        Some("ENEMY")
    } else if layer & layers::ITEM != 0 {
        Some("ITEM")
    } else {
        // Projectiles (P-PROJ, E-PROJ) : trop nombreux, on n'étiquette pas.
        None
    }
}

/// Spawn des `Text2d` flottants au-dessus de chaque zone debug pour rendre
/// la lecture immédiate (PLAYER / AOE / ENEMY / DETECT / MOVE ZONE / CAMERA).
/// Pattern immediate-mode : tous les labels sont despawn au début de la frame
/// puis recréés. Acceptable au volume actuel (~10 labels/frame).
fn draw_zone_labels(
    mut commands: Commands,
    debug: Res<DebugMode>,
    existing: Query<Entity, With<DebugZoneLabel>>,
    collider_q: Query<(&Transform, &Hitbox, &CollisionLayer, Option<&AreaOfEffect>)>,
    detection_q: Query<(&Transform, &PlayerDetection)>,
    zone_q: Query<(&MovementZone, Option<&BoundingRadius>)>,
    camera_q: Query<&Projection>,
    windows: Query<&Window>,
) {
    // 1. Despawn previous frame labels
    for e in existing.iter() {
        if let Ok(mut ec) = commands.get_entity(e) {
            ec.try_despawn();
        }
    }

    if !debug.0 {
        return;
    }

    const FONT_SIZE: f32 = 11.0;
    const Z_LABEL: f32 = 10.0;
    const PADDING: f32 = 6.0;

    // 2. Colliders — labelisés par layer (sauf projectiles, trop nombreux).
    for (transform, hitbox, layer, aoe) in collider_q.iter() {
        let Some(name) = collider_label(layer.0) else { continue };
        // AOE bonus : afficher la durée restante.
        let text = if let Some(aoe) = aoe {
            let remaining = (aoe.lifetime - aoe.elapsed).max(0.0);
            format!("AOE ({:.1}s)", remaining)
        } else {
            name.to_string()
        };
        let y_offset = match &hitbox.0 {
            Shape::Circle(r) => *r + PADDING,
            Shape::Rect { half_length, .. } => *half_length + PADDING,
        };
        commands.spawn((
            Text2d::new(text),
            TextFont {
                font_size: FONT_SIZE,
                ..default()
            },
            TextColor(color_for_layer(layer.0)),
            Transform::from_xyz(
                transform.translation.x,
                transform.translation.y + y_offset,
                Z_LABEL,
            ),
            DebugZoneLabel,
        ));
    }

    // 3. Detection zones (magenta — distinguer des AOE).
    for (transform, detection) in detection_q.iter() {
        let y_offset = match &detection.shape {
            Shape::Circle(r) => *r + PADDING,
            Shape::Rect { half_length, .. } => *half_length + PADDING,
        };
        let color = if detection.inside {
            Color::srgb(1.0, 0.4, 0.4)
        } else {
            Color::srgb(1.0, 0.5, 1.0)
        };
        commands.spawn((
            Text2d::new("DETECT"),
            TextFont {
                font_size: FONT_SIZE,
                ..default()
            },
            TextColor(color),
            Transform::from_xyz(
                transform.translation.x,
                transform.translation.y + y_offset,
                Z_LABEL,
            ),
            DebugZoneLabel,
        ));
    }

    // 4. MovementZone — un seul label (toutes les entités partagent la même
    // zone écran, donc 1 rectangle visuel = 1 label).
    if let Ok(window) = windows.single() {
        let h = window.physical_height() as f32;
        if let Some((zone, bounding)) = zone_q.iter().next() {
            let max_y_raw = (0.5 - zone.margin.y) * h;
            commands.spawn((
                Text2d::new("MOVE ZONE"),
                TextFont {
                    font_size: FONT_SIZE,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.0, 1.0)),
                Transform::from_xyz(0.0, max_y_raw - FONT_SIZE - 2.0, Z_LABEL),
                DebugZoneLabel,
            ));
            if let Some(b) = bounding {
                let eff_max_y = max_y_raw - b.0;
                commands.spawn((
                    Text2d::new("EFFECTIVE ZONE"),
                    TextFont {
                        font_size: FONT_SIZE * 0.85,
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 0.5, 0.8)),
                    Transform::from_xyz(0.0, eff_max_y - FONT_SIZE - 2.0, Z_LABEL),
                    DebugZoneLabel,
                ));
            }
        }
    }

    // 5. Caméra — label au coin haut-gauche de la zone visible.
    for projection in camera_q.iter() {
        if let Projection::Orthographic(ortho) = projection {
            commands.spawn((
                Text2d::new("CAMERA"),
                TextFont {
                    font_size: FONT_SIZE,
                    ..default()
                },
                TextColor(Color::srgb(0.0, 1.0, 0.0)),
                Transform::from_xyz(
                    ortho.area.min.x + 60.0,
                    ortho.area.max.y - FONT_SIZE - 2.0,
                    Z_LABEL,
                ),
                DebugZoneLabel,
            ));
            break;
        }
    }
}
