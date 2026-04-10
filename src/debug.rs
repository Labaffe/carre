//! Mode debug (F1) : affiche un overlay avec FPS, timer, difficulté,
//! dessine les hitboxes de tous les `Hittable` (cercles ou rectangles OBB),
//! et affiche le nom du sprite au-dessus de chaque astéroïde (ex: "x007").

use crate::asteroid::Asteroid;
use crate::boss::{Boss, BossProjectile};
use crate::collision::Hittable;
use crate::difficulty::Difficulty;
use crate::missile::Missile;
use crate::player::Player;
use crate::weapon::HitboxShape;
use crate::MusicMain;
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

fn setup_debug_ui(mut commands: Commands) {
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
            ..default()
        },
        DebugUI,
    ));
}

fn toggle_debug(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut debug: ResMut<DebugMode>,
    mut ui_q: Query<&mut Visibility, With<DebugUI>>,
    mut difficulty: ResMut<crate::difficulty::Difficulty>,
    music_q: Query<Entity, With<MusicMain>>,
) {
    // F2 : sauter à 31 secondes (début du "niveau 2")
    if keyboard.just_pressed(KeyCode::F2) {
        difficulty.elapsed = crate::difficulty::SPAWN_STOP_TIME;
        difficulty.spawning_stopped = true;
        difficulty.charging_played = true;
        difficulty.boom_played = true;
        difficulty.boom_14_played = true;
        difficulty.boom_18_played = true;
        difficulty.boom_22_played = true;
        // Vitesse du background à 31s : déjà en décélération depuis 26.7s (4.3s écoulées)
        let t = (4.3 / 6.0_f32).clamp(0.0, 1.0);
        let bg_speed_at_stop = 150.0 * (1.0 + 8.0 * 3.0);
        difficulty.bg_speed_override = Some(bg_speed_at_stop + (50.0 - bg_speed_at_stop) * t);

        // Couper la musique gradius immédiatement
        for entity in music_q.iter() {
            commands.entity(entity).despawn_recursive();
        }
    }

    if keyboard.just_pressed(KeyCode::F1) {
        debug.0 = !debug.0;
        if let Ok(mut vis) = ui_q.get_single_mut() {
            *vis = if debug.0 {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
}

fn update_debug_ui(
    debug: Res<DebugMode>,
    time: Res<Time>,
    difficulty: Res<Difficulty>,
    mut ui_q: Query<&mut Text, With<DebugUI>>,
) {
    if !debug.0 {
        return;
    }

    let fps = 1.0 / time.delta_seconds();
    let elapsed = difficulty.elapsed;
    let factor = difficulty.factor;

    let minutes = (elapsed / 60.0) as u32;
    let seconds = (elapsed % 60.0) as u32;

    if let Ok(mut text) = ui_q.get_single_mut() {
        text.sections[0].value = format!(
            "[DEBUG]\nFPS        : {:.0}\nTimer      : {:02}:{:02}\nDifficulté : x{:.2}",
            fps, minutes, seconds, factor
        );
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
    // Supprimer les labels dont l'astéroïde n'existe plus
    for (label_entity, label, _, _) in label_q.iter() {
        if asteroid_q.get(label.0).is_err() {
            commands.entity(label_entity).despawn();
        }
    }

    if !debug.0 {
        // Cacher tous les labels
        for (_, _, _, mut vis) in label_q.iter_mut() {
            *vis = Visibility::Hidden;
        }
        return;
    }

    // Mettre à jour la position des labels existants
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

    // Créer les labels pour les nouveaux astéroïdes
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
fn draw_hittable<T: Hittable>(
    gizmos: &mut Gizmos,
    query: &Query<(&Transform, &T)>,
    color: Color,
) {
    for (transform, hittable) in query.iter() {
        let pos = transform.translation.truncate();
        match hittable.hitbox_shape() {
            HitboxShape::Circle(r) => {
                gizmos.circle_2d(pos, r, color);
            }
            HitboxShape::Rect { half_length, half_width } => {
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
    boss_q: Query<(&Transform, &Boss)>,
    boss_proj_q: Query<(&Transform, &BossProjectile)>,
) {
    if !debug.0 {
        return;
    }

    draw_hittable(&mut gizmos, &player_q, Color::GREEN);
    draw_hittable(&mut gizmos, &asteroid_q, Color::RED);
    draw_hittable(&mut gizmos, &missile_q, Color::YELLOW);
    draw_hittable(&mut gizmos, &boss_q, Color::CYAN);
    draw_hittable(&mut gizmos, &boss_proj_q, Color::rgba(1.0, 0.5, 0.0, 1.0));
}
