//! Système d'items : drop, mouvement, ramassage, bombes.
//!
//! Quand une entité avec `DropTable` meurt, un item peut apparaître selon
//! les probabilités configurées. L'item descend lentement et disparaît
//! hors écran. Si le joueur le touche, l'effet se déclenche.
//!
//! Le joueur peut accumuler des bombes et les déclencher avec Espace.
//! La bombe = solution de dernier recours qui nettoie tout :
//! - inflige des dégâts à tous les astéroïdes et ennemis à l'écran
//! - despawn tous les projectiles ennemis
//! - despawn toutes les AOE actives (kamikaze + mine explosions, etc.)

use crate::audio::{Sfx, SfxPlayer};
use crate::enemy::asteroid::Asteroid;
use crate::enemy::enemy::Enemy;
use crate::enemy::kamikaze::Kamikaze;
use crate::fx::explosion::load_frames_from_folder;
use crate::game_manager::state::GameState;
use crate::menu::pause::not_paused;
use crate::geometry::shape::Shape;
use crate::physic::area_of_effect::AreaOfEffect;
use crate::physic::collider::{collider, layers};
use crate::physic::health::Health;
use crate::physic::invulnerable::Invulnerable;
use crate::player::player::Player;
use crate::ui::score::Score;
use crate::weapon::projectile::{Projectile, Team};
use bevy::prelude::*;

pub struct ItemPlugin;

impl Plugin for ItemPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DropEvent>()
            .add_message::<BombEvent>()
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
                    item_pickup_on_overlap,
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
    fn pickup_sound(&self) -> Sfx {
        match self {
            ItemType::Bomb => Sfx::ItemPickup,
            ItemType::BonusScore => Sfx::ItemPickup,
        }
    }
}

// ─── Composants & Ressources ────────────────────────────────────────

/// Un item ramassable qui descend à l'écran.
#[derive(Component)]
#[require(crate::GameplayEntity)]
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
#[derive(Message)]
pub struct DropEvent {
    pub position: Vec3,
    pub table: &'static [(ItemType, f32)],
}

/// Émis quand le joueur déclenche une bombe.
#[derive(Message)]
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
            (
            Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(92.0),
                    left: Val::Px(32.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    ..default()
                },
        ),
            BombUI,
        ))
        .with_children(|parent| {
            // Conteneur des icônes de bombes
            parent
                .spawn((
                    (
            Node {
                            column_gap: Val::Px(6.0),
                            ..default()
                        },
        ),
                    BombIconsContainer,
                ))
                .with_children(|icons_parent| {
                    let bomb_texture = asset_server.load("images/bomb/frame000.png");
                    for i in 0..BOMB_MAX_DISPLAY {
                        icons_parent.spawn((
                            ImageNode::new(bomb_texture.clone()),
                            Node {
                                width: Val::Px(BOMB_ICON_SIZE),
                                height: Val::Px(BOMB_ICON_SIZE),
                                ..default()
                            },
                            Visibility::Hidden,
                            BombIcon(i),
                        ));
                    }
                });

            // Texte clignotant "ESPACE"
            parent.spawn((
                Text::new("[ESPACE]"),
                TextFont { font, font_size: 14.0, ..default() },
                TextColor(Color::WHITE),
                Node::default(),
                BombHintText {
                    timer: Timer::from_seconds(BOMB_HINT_VISIBLE, TimerMode::Once),
                    visible: true,
                },
            ));
        });
}

fn cleanup_bomb_ui(mut commands: Commands, query: Query<Entity, With<BombUI>>) {
    for entity in query.iter() {
        if let Ok(mut e) = commands.get_entity(entity) {
            e.try_despawn();
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
    mut bomb_events: MessageWriter<BombEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut sfx: SfxPlayer,
) {
    if keyboard.just_pressed(KeyCode::Space) && bombs.count > 0 {
        bombs.count -= 1;
        bomb_events.write(BombEvent);

        sfx.play_at(Sfx::PlayerBomb, 3.0);

        // Flash blanc plein écran
        commands.spawn((
            Sprite {
                color: Color::WHITE,
                custom_size: Some(Vec2::new(4000.0, 4000.0)),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, 900.0),
            BombScreenFlash(Timer::from_seconds(BOMB_FLASH_DURATION, TimerMode::Once)),
        ));
    }
}

fn bomb_apply_damage(
    mut commands: Commands,
    mut bomb_events: MessageReader<BombEvent>,
    asteroids: Query<Entity, With<Asteroid>>,
    kamikazes: Query<Entity, With<Kamikaze>>,
    enemies: Query<Entity, (With<Enemy>, Without<Asteroid>, Without<Kamikaze>)>,
    enemy_projectiles: Query<(Entity, &Projectile)>,
    aoes: Query<Entity, With<AreaOfEffect>>,
    mut damage_events: MessageWriter<crate::physic::health::DamageEvent>,
) {
    if bomb_events.read().next().is_none() {
        return;
    }
    bomb_events.read().for_each(drop);

    // 1. Damage : DamageEvent pour asteroids + ennemis non-kamikaze
    //    (apply_damage filtre Invulnerable et applique selon Health).
    for entity in asteroids.iter() {
        damage_events.write(crate::physic::health::DamageEvent {
            target: entity,
            amount: BOMB_DAMAGE_ASTEROID,
            source: None,
        });
    }
    for entity in enemies.iter() {
        damage_events.write(crate::physic::health::DamageEvent {
            target: entity,
            amount: BOMB_DAMAGE_ENEMY,
            source: None,
        });
    }

    // 2. Kamikazes : despawn direct (bypass DamageEvent). Sinon le pipeline
    //    HP=0 → `kamikaze_force_boom_system` insère `KamikazeBoom` →
    //    `kamikaze_boom_system` spawn l'AOE de mort, qui survit au despawn
    //    massif d'AOE ci-dessous (spawn AFTER query). Effet panic button
    //    raté. Trade-off : pas de score ni de drop pour les kamikazes tués
    //    au bomb — assumé pour un "last resort".
    for entity in kamikazes.iter() {
        if let Ok(mut e) = commands.get_entity(entity) {
            e.try_despawn();
        }
    }

    // 3. Despawn brut : projectiles ennemis + AOE actives. Pas de Health, on
    //    ne passe donc pas par DamageEvent — c'est l'effet "panique" du bomb
    //    qui nettoie tout l'écran (solution de dernier recours).
    for (entity, projectile) in enemy_projectiles.iter() {
        if projectile.team == Team::Enemy {
            if let Ok(mut e) = commands.get_entity(entity) {
                e.try_despawn();
            }
        }
    }
    for entity in aoes.iter() {
        if let Ok(mut e) = commands.get_entity(entity) {
            e.try_despawn();
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
        sprite.color = Color::srgba(1.0, 1.0, 1.0, 1.0 - t);

        if flash.0.is_finished() {
            if let Ok(mut e) = commands.get_entity(entity) {
                e.try_despawn();
            }
        }
    }
}

// ─── Items : spawn, animation, mouvement, ramassage ─────────────────

fn process_drop_events(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut events: MessageReader<DropEvent>,
    item_frames: Res<ItemFrames>,
    mut sfx: SfxPlayer,
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
                Sprite {
                    image: first_frame,
                    custom_size: Some(Vec2::splat(ITEM_SPRITE_SIZE)),
                    ..default()
                },
                Transform::from_translation(event.position),
                Droppable { item_type },
                ItemAnim {
                    frames,
                    index: 0,
                    timer: Timer::from_seconds(ITEM_ANIM_FPS, TimerMode::Repeating),
                },
                collider(
                    Shape::Circle(ITEM_PICKUP_RADIUS),
                    layers::ITEM,
                    layers::PLAYER,
                ),
            ));

            sfx.play_at(Sfx::ItemAppear, 3.0);
        }
    }
}

fn animate_items(time: Res<Time>, mut query: Query<(&mut Sprite, &mut ItemAnim)>) {
    for (mut sprite, mut anim) in query.iter_mut() {
        if anim.frames.is_empty() {
            continue;
        }
        anim.timer.tick(time.delta());
        if anim.timer.just_finished() {
            anim.index = (anim.index + 1) % anim.frames.len();
            sprite.image = anim.frames[anim.index].clone();
        }
    }
}

fn move_droppables(time: Res<Time>, mut query: Query<&mut Transform, With<Droppable>>) {
    let dt = time.delta_secs();
    for mut transform in query.iter_mut() {
        transform.translation.y -= ITEM_FALL_SPEED * dt;
    }
}

fn cleanup_offscreen_droppables(
    mut commands: Commands,
    windows: Query<&Window>,
    query: Query<(Entity, &Transform), With<Droppable>>,
) {
    let window = windows.single().unwrap();
    let limit = -window.height() / 2.0 - 50.0;
    for (entity, transform) in query.iter() {
        if transform.translation.y < limit {
            if let Ok(mut e) = commands.get_entity(entity) {
                e.try_despawn();
            }
        }
    }
}

/// Réactif sur `OverlapEvent` : pour chaque overlap player ↔ item, applique
/// l'effet (bomb/score), joue le son, despawn l'item. Multi-pickup possible
/// dans la même frame (chaque event est traité).
fn item_pickup_on_overlap(
    mut commands: Commands,
    mut events: MessageReader<crate::physic::collider::OverlapEvent>,
    droppable_q: Query<&Droppable>,
    mut bombs: ResMut<PlayerBombs>,
    mut score: ResMut<Score>,
    mut sfx: SfxPlayer,
) {
    use crate::physic::collider::layers;
    for ev in events.read() {
        // pick(layer) retourne (entity_avec_ce_layer, autre). On veut l'item.
        let Some((item_e, _player_e)) = ev.pick(layers::ITEM) else { continue };
        let Ok(droppable) = droppable_q.get(item_e) else { continue };

        match droppable.item_type {
            ItemType::Bomb => {
                bombs.count += 1;
            }
            ItemType::BonusScore => {
                score.add(BONUS_SCORE_VALUE);
            }
        }
        sfx.play_at(droppable.item_type.pickup_sound(), 3.0);

        if let Ok(mut e) = commands.get_entity(item_e) {
            e.try_despawn();
        }
    }
}
