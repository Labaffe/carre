//! Système d'items : drop, mouvement, ramassage.
//!
//! Quand une entité avec `DropTable` meurt, un item peut apparaître selon
//! les probabilités configurées. L'item descend lentement et disparaît
//! hors écran. Si le joueur le touche, l'effet se déclenche.

use crate::collision::PLAYER_RADIUS;
use crate::explosion::load_frames_from_folder;
use crate::player::Player;
use crate::state::GameState;
use bevy::prelude::*;

pub struct ItemPlugin;

impl Plugin for ItemPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<DropEvent>()
            .add_systems(Startup, preload_item_frames)
            .add_systems(
                Update,
                (
                    process_drop_events,
                    move_droppables,
                    cleanup_offscreen_droppables,
                    player_pickup,
                    animate_items,
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

// ─── Constantes ─────────────────────────────────────────────────────

const ITEM_FALL_SPEED: f32 = 80.0;
const ITEM_PICKUP_RADIUS: f32 = 30.0;
const ITEM_SPRITE_SIZE: f32 = 48.0;

// ─── Types d'items ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub enum ItemType {
    /// Déclenche le son "level_up.ogg".
    Bomb,
}

impl ItemType {
    fn pickup_sound(&self) -> &'static str {
        match self {
            ItemType::Bomb => "audio/level_up.ogg",
        }
    }
}

// ─── Composants ─────────────────────────────────────────────────────

/// Un item ramassable qui descend à l'écran.
#[derive(Component)]
pub struct Droppable {
    pub item_type: ItemType,
}

/// Animation d'un item : cycle les frames à intervalle régulier.
#[derive(Component)]
struct ItemAnim {
    frames: Vec<Handle<Image>>,
    index: usize,
    timer: Timer,
}

/// Frames préchargées pour chaque type d'item.
#[derive(Resource)]
struct ItemFrames {
    bomb: Vec<Handle<Image>>,
}

/// Table de drop attachée à une entité.
/// Chaque entrée : (type d'item, probabilité entre 0.0 et 1.0).
#[derive(Component)]
pub struct DropTable {
    pub drops: &'static [(ItemType, f32)],
}

// ─── Événement ──────────────────────────────────────────────────────

/// Émis quand une entité avec `DropTable` meurt.
#[derive(Event)]
pub struct DropEvent {
    pub position: Vec3,
    pub table: &'static [(ItemType, f32)],
}

// ─── Systèmes ───────────────────────────────────────────────────────

const ITEM_ANIM_FPS: f32 = 0.12;

fn preload_item_frames(mut commands: Commands, asset_server: Res<AssetServer>) {
    let bomb = load_frames_from_folder(&asset_server, "images/bomb").unwrap_or_default();
    commands.insert_resource(ItemFrames { bomb });
}

fn process_drop_events(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut events: EventReader<DropEvent>,
    item_frames: Res<ItemFrames>,
) {
    for event in events.read() {
        for &(item_type, chance) in event.table {
            if fastrand::f32() > chance {
                continue;
            }

            let frames = match item_type {
                ItemType::Bomb => item_frames.bomb.clone(),
            };

            let first_frame = frames.first().cloned().unwrap_or_default();

            commands.spawn((
                SpriteBundle {
                    texture: first_frame,
                    sprite: Sprite {
                        custom_size: Some(Vec2::splat(ITEM_SPRITE_SIZE)),
                        ..default()
                    },
                    transform: Transform::from_translation(event.position),
                    ..default()
                },
                Droppable { item_type },
                ItemAnim {
                    frames,
                    index: 0,
                    timer: Timer::from_seconds(ITEM_ANIM_FPS, TimerMode::Repeating),
                },
            ));

            // Son générique d'apparition d'item
            commands.spawn(AudioBundle {
                source: asset_server.load("audio/level_up.ogg"),
                settings: PlaybackSettings {
                    volume: bevy::audio::Volume::new(3.0),
                    ..PlaybackSettings::DESPAWN
                },
            });
        }
    }
}

fn animate_items(time: Res<Time>, mut query: Query<(&mut Handle<Image>, &mut ItemAnim)>) {
    for (mut texture, mut anim) in query.iter_mut() {
        if anim.frames.is_empty() {
            continue;
        }
        anim.timer.tick(time.delta());
        if anim.timer.just_finished() {
            anim.index = (anim.index + 1) % anim.frames.len();
            *texture = anim.frames[anim.index].clone();
        }
    }
}

fn move_droppables(time: Res<Time>, mut query: Query<&mut Transform, With<Droppable>>) {
    let dt = time.delta_seconds();
    for mut transform in query.iter_mut() {
        transform.translation.y -= ITEM_FALL_SPEED * dt;
    }
}

fn cleanup_offscreen_droppables(
    mut commands: Commands,
    windows: Query<&Window>,
    query: Query<(Entity, &Transform), With<Droppable>>,
) {
    let window = windows.single();
    let limit = -window.height() / 2.0 - 50.0;
    for (entity, transform) in query.iter() {
        if transform.translation.y < limit {
            commands.entity(entity).despawn();
        }
    }
}

fn player_pickup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    player_q: Query<&Transform, With<Player>>,
    item_q: Query<(Entity, &Transform, &Droppable)>,
) {
    let Ok(player_transform) = player_q.get_single() else {
        return;
    };
    let player_pos = player_transform.translation;

    for (entity, item_transform, droppable) in item_q.iter() {
        let dist = player_pos.distance(item_transform.translation);
        if dist > PLAYER_RADIUS + ITEM_PICKUP_RADIUS {
            continue;
        }

        // Appliquer l'effet
        match droppable.item_type {
            ItemType::Bomb => {
                // Pour l'instant, juste le son
            }
        }

        commands.spawn(AudioBundle {
            source: asset_server.load(droppable.item_type.pickup_sound()),
            settings: PlaybackSettings {
                volume: bevy::audio::Volume::new(3.0),
                ..PlaybackSettings::DESPAWN
            },
        });

        commands.entity(entity).despawn();
    }
}
