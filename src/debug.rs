//! Mode debug (F1) : affiche un overlay avec FPS, timer, difficulté,
//! dessine les hitboxes de tous les `Hittable` (cercles ou rectangles OBB),
//! affiche le nom du sprite au-dessus de chaque astéroïde (ex: "x007"),
//! et affiche la timeline du niveau avec les liens de causalité.

use crate::MusicMain;
use crate::asteroid::Asteroid;
use crate::boss::{BossCharge, BossMarker};
use crate::green_ufo::GreenUFOMarker;
use crate::collision::Hittable;
use crate::difficulty::Difficulty;
use crate::enemy::{Enemy, EnemyProjectile, EnemyState, PatternIndex, PatternTimer};
use crate::level::{LevelRunner, Trigger};
use crate::missile::Missile;
use crate::player::{Player, PlayerLives};
use crate::score::Score;
use crate::weapon::HitboxShape;
use bevy::prelude::*;

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DebugMode(false))
            .add_systems(Startup, setup_debug_ui)
            .add_systems(
                Update,
                (
                    toggle_debug,
                    draw_hitboxes,
                    update_debug_ui,
                    update_debug_level_ui,
                    manage_asteroid_labels,
                ),
            );
    }
}

#[derive(Component)]
struct AsteroidLabel(Entity);

#[derive(Resource)]
pub struct DebugMode(pub bool);

#[derive(Component)]
struct DebugUI;

#[derive(Component)]
struct DebugLevelUI;

fn setup_debug_ui(mut commands: Commands) {
    // Panneau gauche : infos générales
    commands.spawn((
        TextBundle {
            text: Text::from_sections([TextSection::new(
                "",
                TextStyle {
                    font_size: 16.0,
                    color: Color::WHITE,
                    ..default()
                },
            )]),
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                left: Val::Px(10.0),
                ..default()
            },
            visibility: Visibility::Hidden,
            z_index: ZIndex::Global(100),
            ..default()
        },
        DebugUI,
    ));

    // Panneau droit : timeline du niveau
    commands.spawn((
        TextBundle {
            text: Text::from_sections([TextSection::new(
                "",
                TextStyle {
                    font_size: 14.0,
                    color: Color::WHITE,
                    ..default()
                },
            )]),
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                right: Val::Px(10.0),
                ..default()
            },
            visibility: Visibility::Hidden,
            z_index: ZIndex::Global(100),
            ..default()
        },
        DebugLevelUI,
    ));
}

fn toggle_debug(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut debug: ResMut<DebugMode>,
    mut ui_q: Query<&mut Visibility, (With<DebugUI>, Without<DebugLevelUI>)>,
    mut level_ui_q: Query<&mut Visibility, (With<DebugLevelUI>, Without<DebugUI>)>,
    mut difficulty: ResMut<crate::difficulty::Difficulty>,
    music_q: Query<Entity, With<MusicMain>>,
) {
    if keyboard.just_pressed(KeyCode::F2) {
        difficulty.spawning_stopped = true;
        difficulty.active_spawners.clear();
        difficulty.factor = 7.5;
        let t = (4.3 / 6.0_f32).clamp(0.0, 1.0);
        let bg_speed_at_stop = 150.0 * (1.0 + 8.0 * 3.0);
        difficulty.bg_speed_override = Some(bg_speed_at_stop + (50.0 - bg_speed_at_stop) * t);

        for entity in music_q.iter() {
            commands.entity(entity).despawn_recursive();
        }
    }

    if keyboard.just_pressed(KeyCode::F1) {
        debug.0 = !debug.0;
        let new_vis = if debug.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if let Ok(mut vis) = ui_q.get_single_mut() {
            *vis = new_vis;
        }
        if let Ok(mut vis) = level_ui_q.get_single_mut() {
            *vis = new_vis;
        }
    }
}

fn update_debug_ui(
    debug: Res<DebugMode>,
    time: Res<Time>,
    difficulty: Res<Difficulty>,
    lives: Res<PlayerLives>,
    score: Res<Score>,
    mut ui_q: Query<&mut Text, With<DebugUI>>,
    player_q: Query<&Transform, With<Player>>,
    enemy_q: Query<
        (
            &Enemy,
            &Transform,
            Option<&BossMarker>,
            Option<&GreenUFOMarker>,
            Option<&PatternIndex>,
            Option<&PatternTimer>,
            Option<&BossCharge>,
        ),
    >,
    asteroid_q: Query<&Asteroid>,
    missile_q: Query<&Missile>,
) {
    if !debug.0 {
        return;
    }

    let fps = 1.0 / time.delta_seconds();
    let elapsed = difficulty.elapsed;
    let factor = difficulty.factor;

    let minutes = (elapsed / 60.0) as u32;
    let seconds = (elapsed % 60.0) as u32;

    let player_pos = player_q
        .get_single()
        .map(|t| format!("({:.0}, {:.0})", t.translation.x, t.translation.y))
        .unwrap_or_else(|_| "N/A".to_string());

    let mut enemy_lines = String::new();
    for (enemy, transform, boss, green_ufo, pat_idx, pat_timer, charge) in enemy_q.iter() {
        let name = if boss.is_some() {
            "Boss"
        } else if green_ufo.is_some() {
            "GreenUFO"
        } else {
            "Enemy"
        };
        let pos = format!("({:.0}, {:.0})", transform.translation.x, transform.translation.y);

        let state_str = match &enemy.state {
            EnemyState::Entering => "Entering".to_string(),
            EnemyState::Flexing => "Flexing".to_string(),
            EnemyState::Idle => "Idle".to_string(),
            EnemyState::Active(idx) => {
                let phase = &enemy.phases[*idx];
                let pattern_info = if let Some(pi) = pat_idx {
                    let p_idx = pi.0 % phase.patterns.len();
                    let p = &phase.patterns[p_idx];
                    if charge.is_some() {
                        format!(">> {} (charging)", p.name)
                    } else {
                        let remaining = pat_timer
                            .map(|t| t.0.duration().as_secs_f32() - t.0.elapsed_secs())
                            .unwrap_or(0.0);
                        format!("{} ({:.1}s)", p.name, remaining)
                    }
                } else {
                    "?".to_string()
                };
                format!("Active(phase {}) | {}", idx, pattern_info)
            }
            EnemyState::Transitioning(idx) => format!("Transitioning → phase {}", idx),
            EnemyState::Dying => {
                let remaining = enemy.anim_timer.duration().as_secs_f32() - enemy.anim_timer.elapsed_secs();
                format!("Dying ({:.1}s)", remaining)
            }
            EnemyState::Dead => "Dead".to_string(),
        };

        enemy_lines.push_str(&format!(
            "\n  {} {} | HP {}/{} | {}", name, pos, enemy.health, enemy.max_health, state_str
        ));
    }

    let asteroid_count = asteroid_q.iter().count();
    let missile_count = missile_q.iter().count();

    if let Ok(mut text) = ui_q.get_single_mut() {
        text.sections[0].value = format!(
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
             --- Ennemis ---{}\n\
             \n\
             F1 : Debug Mode ON/OFF\n\
             F2 : Skip asteroides\n\
             F3 : Skip au boss",
            fps, minutes, seconds, factor,
            lives.lives,
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
    mut ui_q: Query<&mut Text, With<DebugLevelUI>>,
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

    let mut lines = String::from("--- Timeline Niveau 1 ---\n\n");

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
        lines.push_str("NIVEAU TERMINE - Boss actif\n");
    }

    if let Ok(mut text) = ui_q.get_single_mut() {
        text.sections[0].value = lines;
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
            commands.entity(label_entity).despawn();
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
        let name = format!("x{:03}", asteroid.texture_index);
        commands.spawn((
            Text2dBundle {
                text: Text::from_section(
                    name,
                    TextStyle {
                        font_size: 14.0,
                        color: Color::rgba(1.0, 1.0, 1.0, 0.7),
                        ..default()
                    },
                ),
                transform: Transform::from_xyz(
                    transform.translation.x,
                    transform.translation.y + asteroid.radius + 15.0,
                    10.0,
                ),
                ..default()
            },
            AsteroidLabel(entity),
        ));
    }
}

/// Dessine la hitbox d'un Hittable via gizmos.
fn draw_hittable<T: Hittable>(gizmos: &mut Gizmos, query: &Query<(&Transform, &T)>, color: Color) {
    for (transform, hittable) in query.iter() {
        let pos = transform.translation.truncate();
        match hittable.hitbox_shape() {
            HitboxShape::Circle(r) => {
                gizmos.circle_2d(pos, r, color);
            }
            HitboxShape::Rect {
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

fn draw_hitboxes(
    debug: Res<DebugMode>,
    mut gizmos: Gizmos,
    player_q: Query<(&Transform, &Player)>,
    asteroid_q: Query<(&Transform, &Asteroid)>,
    missile_q: Query<(&Transform, &Missile)>,
    enemy_q: Query<(&Transform, &Enemy)>,
    enemy_proj_q: Query<(&Transform, &EnemyProjectile)>,
) {
    if !debug.0 {
        return;
    }

    draw_hittable(&mut gizmos, &player_q, Color::GREEN);
    draw_hittable(&mut gizmos, &asteroid_q, Color::RED);
    draw_hittable(&mut gizmos, &missile_q, Color::YELLOW);
    draw_hittable(&mut gizmos, &enemy_q, Color::CYAN);
    draw_hittable(&mut gizmos, &enemy_proj_q, Color::rgba(1.0, 0.5, 0.0, 1.0));
}
