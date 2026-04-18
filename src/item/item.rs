//! Système d'items : drop, mouvement, ramassage, bombes.
//!
//! Quand une entité avec `DropTable` meurt, un item peut apparaître selon
//! les probabilités configurées. L'item descend lentement et disparaît
//! hors écran. Si le joueur le touche, l'effet se déclenche.
//!
//! Le joueur peut accumuler des bombes et les déclencher avec Espace.
//! La bombe inflige des dégâts à tous les astéroïdes et ennemis à l'écran.

use crate::enemy::asteroid::Asteroid;
use crate::enemy::enemy::{Enemy, EnemyState};
use crate::fx::explosion::load_frames_from_folder;
use crate::game_manager::state::GameState;
use crate::menu::pause::not_paused;
use crate::physic::collision::PLAYER_RADIUS;
use crate::physic::health::Health;
use crate::player::player::Player;
use crate::ui::score::Score;
use bevy::prelude::*;

pub struct ItemPlugin;

impl Plugin for ItemPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<DropEvent>()
            .add_event::<BombEvent>()
            .init_resource::<PlayerBombs>()
            .add_systems(Startup, preload_item_frames)
            .add_systems(OnEnter(GameState::Playing), (setup_bomb_ui, reset_bombs))
            .add_systems(OnExit(GameState::Playing), cleanup_bomb_ui)
            .add_systems(
                Update,
                (
                    process_drop_events,
                    move_droppables,
                    cleanup_offscreen_droppables,
                    player_pickup,
                    animate_items,
                    bomb_input,
                    bomb_apply_damage,
                    bomb_screen_flash,
                    update_bomb_ui,
                    blink_bomb_hint,
                )
                    .run_if(in_state(GameState::Playing))
                    .run_if(not_paused),
            );
    }
}

// ─── Constantes ─────────────────────────────────────────────────────

const ITEM_FALL_SPEED: f32 = 300.0;
const ITEM_PICKUP_RADIUS: f32 = 30.0;
const ITEM_SPRITE_SIZE: f32 = 72.0;

/// Dégâts infligés par la bombe aux astéroïdes.
const BOMB_DAMAGE_ASTEROID: i32 = 999;
/// Dégâts infligés par la bombe aux ennemis.
const BOMB_DAMAGE_ENEMY: i32 = 50;
/// Durée du flash blanc à l'écran (secondes).
const BOMB_FLASH_DURATION: f32 = 0.4;
/// Taille des icônes de bombe dans l'UI.
const BOMB_ICON_SIZE: f32 = 56.0;
/// Nombre max de bombes affichées dans l'UI.
const BOMB_MAX_DISPLAY: i32 = 10;
/// Durée visible du texte clignotant (secondes).
const BOMB_HINT_VISIBLE: f32 = 0.7;
/// Durée invisible du texte clignotant (secondes).
const BOMB_HINT_HIDDEN: f32 = 0.3;
/// Bonus de score accordé par l'item BonusScore.
const BONUS_SCORE_VALUE: i32 = 50;

// ─── Types d'items ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub enum ItemType {
    Bomb,
    BonusScore,
}

impl ItemType {
    fn pickup_sound(&self) -> &'static str {
        match self {
            ItemType::Bomb => "audio/sfx/level_up.ogg",
            ItemType::BonusScore => "audio/sfx/level_up.ogg",
        }
    }
}

// ─── Composants & Ressources ────────────────────────────────────────

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
    bonus_score: Vec<Handle<Image>>,
}

/// Table de drop attachée à une entité.
/// Chaque entrée : (type d'item, probabilité entre 0.0 et 1.0).
#[derive(Component)]
pub struct DropTable {
    pub drops: &'static [(ItemType, f32)],
}

/// Compteur de bombes du joueur.
#[derive(Resource)]
pub struct PlayerBombs {
    pub count: i32,
}

impl Default for PlayerBombs {
    fn default() -> Self {
        Self { count: 0 }
    }
}

// ─── Événements ─────────────────────────────────────────────────────

/// Émis quand une entité avec `DropTable` meurt.
#[derive(Event)]
pub struct DropEvent {
    pub position: Vec3,
    pub table: &'static [(ItemType, f32)],
}

/// Émis quand le joueur déclenche une bombe.
#[derive(Event)]
pub struct BombEvent;

// ─── Composants UI ──────────────────────────────────────────────────

/// Conteneur racine de l'UI des bombes.
#[derive(Component)]
struct BombUI;

/// Conteneur des icônes de bombes.
#[derive(Component)]
struct BombIconsContainer;

/// Icône individuelle de bombe dans l'UI.
#[derive(Component)]
struct BombIcon(i32);

/// Texte "ESPACE" qui clignote.
#[derive(Component)]
struct BombHintText {
    timer: Timer,
    visible: bool,
}

/// Flash blanc plein écran quand une bombe explose.
#[derive(Component)]
struct BombScreenFlash(Timer);

// ─── Systèmes ───────────────────────────────────────────────────────

const ITEM_ANIM_FPS: f32 = 0.12;

fn preload_item_frames(mut commands: Commands, asset_server: Res<AssetServer>) {
    let bomb = load_frames_from_folder(&asset_server, "images/bomb").unwrap_or_default();
    let bonus_score =
        load_frames_from_folder(&asset_server, "images/bonus_score").unwrap_or_default();
    commands.insert_resource(ItemFrames { bomb, bonus_score });
}

fn reset_bombs(mut bombs: ResMut<PlayerBombs>) {
    *bombs = PlayerBombs::default();
}

// ─── UI des bombes ──────────────────────────────────────────────────

fn setup_bomb_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/PressStart2P-Regular.ttf");

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    top: Val::Px(92.0),
                    left: Val::Px(32.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    ..default()
                },
                ..default()
            },
            BombUI,
        ))
        .with_children(|parent| {
            // Conteneur des icônes de bombes
            parent
                .spawn((
                    NodeBundle {
                        style: Style {
                            column_gap: Val::Px(6.0),
                            ..default()
                        },
                        ..default()
                    },
                    BombIconsContainer,
                ))
                .with_children(|icons_parent| {
                    let bomb_texture = asset_server.load("images/bomb/frame000.png");
                    for i in 0..BOMB_MAX_DISPLAY {
                        icons_parent.spawn((
                            ImageBundle {
                                image: UiImage::new(bomb_texture.clone()),
                                style: Style {
                                    width: Val::Px(BOMB_ICON_SIZE),
                                    height: Val::Px(BOMB_ICON_SIZE),
                                    ..default()
                                },
                                visibility: Visibility::Hidden,
                                ..default()
                            },
                            BombIcon(i),
                        ));
                    }
                });

            // Texte clignotant "ESPACE"
            parent.spawn((
                TextBundle::from_section(
                    "[ESPACE]",
                    TextStyle {
                        font,
                        font_size: 14.0,
                        color: Color::WHITE,
                    },
                )
                .with_style(Style { ..default() }),
                BombHintText {
                    timer: Timer::from_seconds(BOMB_HINT_VISIBLE, TimerMode::Once),
                    visible: true,
                },
            ));
        });
}

fn cleanup_bomb_ui(mut commands: Commands, query: Query<Entity, With<BombUI>>) {
    for entity in query.iter() {
        if let Some(e) = commands.get_entity(entity) {
            e.despawn_recursive();
        }
    }
}

fn update_bomb_ui(
    bombs: Res<PlayerBombs>,
    mut icons: Query<(&BombIcon, &mut Visibility), Without<BombHintText>>,
    mut hint: Query<&mut Visibility, With<BombHintText>>,
) {
    // Mettre à jour la visibilité des icônes
    for (icon, mut vis) in icons.iter_mut() {
        if icon.0 < bombs.count {
            *vis = Visibility::Visible;
        } else {
            *vis = Visibility::Hidden;
        }
    }

    // Cacher le texte si aucune bombe
    if bombs.count == 0 {
        for mut vis in hint.iter_mut() {
            *vis = Visibility::Hidden;
        }
    }
}

fn blink_bomb_hint(
    time: Res<Time>,
    bombs: Res<PlayerBombs>,
    mut query: Query<(&mut Visibility, &mut BombHintText)>,
) {
    if bombs.count == 0 {
        return;
    }

    for (mut vis, mut hint) in query.iter_mut() {
        hint.timer.tick(time.delta());
        if hint.timer.just_finished() {
            hint.visible = !hint.visible;
            let next_duration = if hint.visible {
                BOMB_HINT_VISIBLE
            } else {
                BOMB_HINT_HIDDEN
            };
            hint.timer = Timer::from_seconds(next_duration, TimerMode::Once);
        }

        *vis = if hint.visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

// ─── Input & déclenchement de la bombe ──────────────────────────────

fn bomb_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut bombs: ResMut<PlayerBombs>,
    mut bomb_events: EventWriter<BombEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    if keyboard.just_pressed(KeyCode::Space) && bombs.count > 0 {
        bombs.count -= 1;
        bomb_events.send(BombEvent);

        // Son de bombe
        commands.spawn(AudioBundle {
            source: asset_server.load("audio/sfx/bomb.ogg"),
            settings: PlaybackSettings {
                volume: bevy::audio::Volume::new(3.0),
                ..PlaybackSettings::DESPAWN
            },
        });

        // Flash blanc plein écran
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::WHITE,
                    custom_size: Some(Vec2::new(4000.0, 4000.0)),
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 0.0, 900.0),
                ..default()
            },
            BombScreenFlash(Timer::from_seconds(BOMB_FLASH_DURATION, TimerMode::Once)),
        ));
    }
}

fn bomb_apply_damage(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut bomb_events: EventReader<BombEvent>,
    mut asteroids: Query<(Entity, &Transform, &Asteroid, &mut Health, Option<&DropTable>)>,
    mut enemies: Query<(&Enemy, &mut Health), Without<Asteroid>>,
    mut drop_events: EventWriter<DropEvent>,
    difficulty: Res<crate::game_manager::difficulty::Difficulty>,
) {
    if bomb_events.read().next().is_none() {
        return;
    }
    // Consommer tous les événements restants
    bomb_events.read().for_each(drop);

    // Dégâts à tous les astéroïdes — les tuer directement avec explosion
    for (entity, transform, asteroid, mut health, drop_table) in asteroids.iter_mut() {
        health.take_damage(BOMB_DAMAGE_ASTEROID);
        if health.is_dead() {
            crate::fx::explosion::spawn_explosion(
                &mut commands,
                &asset_server,
                transform.translation,
                asteroid.size,
                asteroid.texture_index,
                asteroid.base_velocity * difficulty.factor,
                transform.rotation,
            );
            if let Some(table) = drop_table {
                drop_events.send(DropEvent {
                    position: transform.translation,
                    table: table.drops,
                });
            }
            if let Some(mut e) = commands.get_entity(entity) {
                e.despawn();
            }
        }
    }

    // Dégâts à tous les ennemis actifs (le framework enemy gère la mort automatiquement)
    for (enemy, mut health) in enemies.iter_mut() {
        if matches!(enemy.state, EnemyState::Active(_)) {
            health.take_damage(BOMB_DAMAGE_ENEMY);
        }
    }
}

fn bomb_screen_flash(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Sprite, &mut BombScreenFlash)>,
) {
    for (entity, mut sprite, mut flash) in query.iter_mut() {
        flash.0.tick(time.delta());
        let t = flash.0.fraction();
        // Fade out : blanc opaque → transparent
        sprite.color = Color::rgba(1.0, 1.0, 1.0, 1.0 - t);

        if flash.0.finished() {
            if let Some(mut e) = commands.get_entity(entity) {
                e.despawn();
            }
        }
    }
}

// ─── Items : spawn, animation, mouvement, ramassage ─────────────────

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
                ItemType::BonusScore => item_frames.bonus_score.clone(),
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
                source: asset_server.load("audio/sfx/level_up.ogg"),
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
            if let Some(mut e) = commands.get_entity(entity) {
                e.despawn();
            }
        }
    }
}

fn player_pickup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    player_q: Query<&Transform, With<Player>>,
    item_q: Query<(Entity, &Transform, &Droppable)>,
    mut bombs: ResMut<PlayerBombs>,
    mut score: ResMut<Score>,
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
                bombs.count += 1;
            }
            ItemType::BonusScore => {
                score.add(BONUS_SCORE_VALUE);
            }
        }

        commands.spawn(AudioBundle {
            source: asset_server.load(droppable.item_type.pickup_sound()),
            settings: PlaybackSettings {
                volume: bevy::audio::Volume::new(3.0),
                ..PlaybackSettings::DESPAWN
            },
        });

        if let Some(mut e) = commands.get_entity(entity) {
            e.despawn();
        }
    }
}
