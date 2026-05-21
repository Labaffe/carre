//! Joueur : spawn, mouvement ZQSD, rotation vers le réticule.
//!
//! Vitesse fixe (`PLAYER_SPEED`). La progression du joueur sera ultérieurement
//! pilotée par le deckbuilding (cartes qui modifient vitesse, arme, etc.).
//! L'ancien système de phases temporelles (Phase1/2/3 via timer + boss music)
//! a été retiré.

use crate::audio::{Sfx, SfxPlayer};
use crate::game_manager::difficulty::BoomEvent;
use crate::game_manager::state::GameState;
use crate::geometry::shape::Shape;
use crate::level::level::{LevelConfig, LevelSetupSet};
use crate::menu::pause::not_paused;
use crate::physic::collider::{collider, layers};
use crate::physic::health::Health;
use crate::physic::invulnerable::Invulnerable;
use crate::ui::crosshair::Crosshair;
use crate::weapon::weapon::Weapon;
use bevy::prelude::*;

// ─── Constantes ────────────────────────────────────────────────────

/// Nombre de vies au départ. `Health` avec ce nombre de PV maximum (un hit = 1 PV).
pub const PLAYER_MAX_LIVES: i32 = 3;
/// Durée d'invincibilité après un hit (secondes).
pub const INVINCIBLE_DURATION: f32 = 2.0;
/// Fréquence de clignotement pendant l'invincibilité (Hz).
const INVINCIBLE_BLINK_RATE: f32 = 3.0;
/// Vitesse de base du joueur (px/s). Constante tant que le deckbuilding
/// n'intervient pas — bande "MATCHED" entre les ennemis lents et les
/// projectiles, ratio ~1.14× kamikaze base, ~0.8× boss charge.
pub const PLAYER_SPEED: f32 = 400.0;
/// Marge bord d'écran pour empêcher le joueur de sortir.
const PLAYER_MARGIN: f32 = 64.0;
/// Taille du sprite du joueur (carré, px).
const PLAYER_SPRITE_SIZE: f32 = 128.0;
/// Rayon de la hitbox du joueur (px). ~70% de la demi-taille du sprite.
const PLAYER_HITBOX_RADIUS: f32 = 45.0;
/// Durée du flash blanc autour du joueur lors d'un boom.
const BOOM_FLASH_DURATION: f32 = 0.25;
/// Distance maximale d'un dash (px). Si le réticule est plus loin, le dash
/// est plafonné à cette distance dans la direction visée.
const DASH_MAX_DISTANCE: f32 = 350.0;
/// Durée d'un dash (s). Très court — l'effet "blink" qui rend invulnérable
/// (via insert/remove de `Invulnerable` sur la même fenêtre).
const DASH_DURATION: f32 = 0.14;
/// Cooldown du dash (s). Long pour forcer le joueur à choisir son moment.
const DASH_COOLDOWN: f32 = 5.0;
/// Largeur de la barre de cooldown en UI (px).
const DASH_UI_BAR_WIDTH: f32 = 120.0;
/// Hauteur de la barre de cooldown en UI (px).
const DASH_UI_BAR_HEIGHT: f32 = 8.0;

// ─── Composants ────────────────────────────────────────────────────

#[derive(Component)]
#[require(crate::GameplayEntity)]
pub struct Player;

/// Invincibilité temporaire après un hit.
#[derive(Component)]
pub struct Invincible(pub Timer);

/// Marqueur pour le conteneur UI des vies.
#[derive(Component)]
pub struct LivesUI;

/// Marqueur individuel pour chaque icône de vie.
#[derive(Component)]
struct LifeIcon(i32);

/// Flash blanc autour du vaisseau lors d'un boom.
#[derive(Component)]
struct BoomFlash(Timer);

/// Composant inséré sur le joueur pendant un dash en cours. Lerp entre
/// `start` et `target` sur `DASH_DURATION`. Tant que ce composant est là :
/// déplacement, tir et rotation sont bloqués (via `Without<Dashing>` dans
/// les queries des systèmes correspondants).
#[derive(Component)]
pub struct Dashing {
    pub start: Vec2,
    pub target: Vec2,
    pub elapsed: f32,
}

/// Ressource globale qui suit le cooldown du dash. Timer Once. Quand
/// `is_finished()` → dash dispo. `reset()` au déclenchement → timer ré-tick
/// pendant `DASH_COOLDOWN` secondes avant que le dash soit re-dispo.
#[derive(Resource)]
pub struct DashCooldown {
    pub timer: Timer,
}

impl Default for DashCooldown {
    fn default() -> Self {
        // État initial = prêt : on tick immédiatement le timer à sa durée
        // complète pour que `is_finished()` soit true au spawn.
        let mut timer = Timer::from_seconds(DASH_COOLDOWN, TimerMode::Once);
        timer.tick(std::time::Duration::from_secs_f32(DASH_COOLDOWN));
        Self { timer }
    }
}

/// Marqueurs UI pour la jauge de dash.
#[derive(Component)]
struct DashUI;
#[derive(Component)]
struct DashUIText;
#[derive(Component)]
struct DashUIBar;

// ─── Plugin ────────────────────────────────────────────────────────

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DashCooldown>()
            .add_systems(
                OnEnter(GameState::Playing),
                (setup_player, setup_lives_ui, setup_dash_ui, reset_dash_cooldown)
                    .after(LevelSetupSet),
            )
            .add_systems(
                OnExit(GameState::Playing),
                (cleanup_lives_ui, cleanup_dash_ui),
            )
            .add_systems(
                Update,
                (
                    movement,
                    rotate_towards_crosshair,
                    boom_flash_trigger,
                    boom_flash_update,
                    update_invincibility,
                    update_lives_ui,
                    dash_input,
                    update_dash,
                    dash_cooldown_tick,
                    update_dash_ui,
                )
                    .run_if(in_state(GameState::Playing))
                    .run_if(not_paused),
            );
    }
}

// ─── Spawn ─────────────────────────────────────────────────────────

fn setup_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    windows: Query<&Window>,
    config: Res<LevelConfig>,
) {
    let window = windows.single().unwrap();
    let half_h = window.height() / 2.0;
    spawn_player(
        &mut commands,
        &asset_server,
        -half_h * 0.5,
        config.player_ship,
    );
}

pub fn spawn_player(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    start_y: f32,
    ship_sprite: &'static str,
) {
    commands.spawn((
        Sprite {
            image: asset_server.load(ship_sprite),
            custom_size: Some(Vec2::splat(PLAYER_SPRITE_SIZE)),
            ..default()
        },
        Transform::from_xyz(0.0, start_y, 0.5),
        Player,
        Health::new(PLAYER_MAX_LIVES),
        Weapon::default(),
        collider(
            Shape::Circle(PLAYER_HITBOX_RADIUS),
            layers::PLAYER,
            layers::ENEMY
                | layers::ASTEROID
                | layers::ENEMY_PROJECTILE
                | layers::AOE
                | layers::ITEM,
        ),
    ));
}

// ─── Mouvement ─────────────────────────────────────────────────────

fn movement(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, (With<Player>, Without<Dashing>)>,
    windows: Query<&Window>,
) {
    let window = windows.single().unwrap();
    let half_w = window.width() / 2.0 - PLAYER_MARGIN;
    let half_h = window.height() / 2.0 - PLAYER_MARGIN;

    let Ok(mut transform) = query.single_mut() else { return; };
    let mut direction = Vec3::ZERO;

    if keyboard.pressed(KeyCode::KeyW) { direction.y += 1.0; }
    if keyboard.pressed(KeyCode::KeyS) { direction.y -= 1.0; }
    if keyboard.pressed(KeyCode::KeyA) { direction.x -= 1.0; }
    if keyboard.pressed(KeyCode::KeyD) { direction.x += 1.0; }

    transform.translation += direction.normalize_or_zero() * PLAYER_SPEED * time.delta_secs();
    transform.translation.x = transform.translation.x.clamp(-half_w, half_w);
    transform.translation.y = transform.translation.y.clamp(-half_h, half_h);
}

// ─── Rotation vers le réticule ─────────────────────────────────────

fn rotate_towards_crosshair(
    crosshair_q: Query<&Transform, (With<Crosshair>, Without<Player>)>,
    mut player_q: Query<&mut Transform, (With<Player>, Without<Crosshair>, Without<Dashing>)>,
) {
    let Ok(crosshair_tf) = crosshair_q.single() else { return };
    let Ok(mut player_transform) = player_q.single_mut() else { return };

    let direction = crosshair_tf.translation - player_transform.translation;
    let angle = direction.y.atan2(direction.x) - std::f32::consts::FRAC_PI_2;
    player_transform.rotation = Quat::from_rotation_z(angle);
}

// ─── Flash blanc au boom ────────────────────────────────────────────

fn boom_flash_trigger(
    mut commands: Commands,
    mut boom_events: MessageReader<BoomEvent>,
    player_q: Query<Entity, With<Player>>,
) {
    if boom_events.read().next().is_none() {
        return;
    }
    boom_events.read().for_each(drop);

    if let Ok(entity) = player_q.single() {
        commands
            .entity(entity)
            .insert(BoomFlash(Timer::from_seconds(
                BOOM_FLASH_DURATION,
                TimerMode::Once,
            )));
    }
}

fn boom_flash_update(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Sprite, &mut BoomFlash), With<Player>>,
) {
    for (entity, mut sprite, mut flash) in query.iter_mut() {
        flash.0.tick(time.delta());
        let t = flash.0.fraction();

        if flash.0.is_finished() {
            sprite.color = Color::WHITE;
            commands.entity(entity).remove::<BoomFlash>();
        } else {
            let intensity = 1.0 + (1.0 - t) * 8.0;
            sprite.color = Color::srgba(intensity, intensity, intensity, 1.0);
        }
    }
}

// ─── Invincibilité ────────────────────────────────────────────────

fn update_invincibility(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Sprite, &mut Invincible), With<Player>>,
) {
    for (entity, mut sprite, mut inv) in query.iter_mut() {
        inv.0.tick(time.delta());

        if inv.0.is_finished() {
            sprite.color = Color::WHITE;
            commands.entity(entity).remove::<Invincible>();
        } else {
            let blink =
                (inv.0.elapsed_secs() * INVINCIBLE_BLINK_RATE * std::f32::consts::TAU).sin();
            let alpha = if blink > 0.0 { 1.0 } else { 0.0 };
            sprite.color = Color::srgba(1.0, 1.0, 1.0, alpha);
        }
    }
}

// ─── UI des vies ──────────────────────────────────────────────────

fn setup_lives_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    config: Res<LevelConfig>,
) {
    let texture = asset_server.load(config.player_ship);

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(20.0),
                left: Val::Px(20.0),
                column_gap: Val::Px(12.0),
                ..default()
            },
            LivesUI,
        ))
        .with_children(|parent| {
            for i in 0..PLAYER_MAX_LIVES {
                parent.spawn((
                    ImageNode::new(texture.clone()),
                    Node {
                        width: Val::Px(64.0),
                        height: Val::Px(64.0),
                        ..default()
                    },
                    LifeIcon(i),
                ));
            }
        });
}

fn update_lives_ui(
    player_q: Query<&Health, With<Player>>,
    mut icons: Query<(&LifeIcon, &mut Visibility)>,
) {
    let current_lives = player_q.single().map(|h| h.current).unwrap_or(0);
    for (icon, mut vis) in icons.iter_mut() {
        if icon.0 < current_lives {
            *vis = Visibility::Visible;
        } else {
            *vis = Visibility::Hidden;
        }
    }
}

fn cleanup_lives_ui(mut commands: Commands, query: Query<Entity, With<LivesUI>>) {
    for entity in query.iter() {
        if let Ok(mut e) = commands.get_entity(entity) {
            e.try_despawn();
        }
    }
}

// ─── Dash : input, motion, cooldown ─────────────────────────────────

/// Reset du cooldown à chaque entrée en Playing (sinon le timer hérité d'une
/// partie précédente reste en cours).
fn reset_dash_cooldown(mut cooldown: ResMut<DashCooldown>) {
    *cooldown = DashCooldown::default();
}

/// Déclenche un dash si :
/// - Espace pressé
/// - cooldown prêt
/// - aucun dash déjà en cours (Without<Dashing> dans la query)
///
/// La cible = position du réticule au moment du clic, plafonnée à
/// `DASH_MAX_DISTANCE` dans la direction visée.
fn dash_input(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut cooldown: ResMut<DashCooldown>,
    mut sfx: SfxPlayer,
    crosshair_q: Query<&Transform, (With<Crosshair>, Without<Player>)>,
    player_q: Query<(Entity, &Transform), (With<Player>, Without<Dashing>, Without<Crosshair>)>,
) {
    if !keyboard.just_pressed(KeyCode::Space) {
        return;
    }
    if !cooldown.timer.is_finished() {
        return;
    }
    let Ok((player_e, player_tf)) = player_q.single() else { return };
    let Ok(crosshair_tf) = crosshair_q.single() else { return };

    let start = player_tf.translation.truncate();
    let crosshair_pos = crosshair_tf.translation.truncate();
    let to_crosshair = crosshair_pos - start;
    let dist = to_crosshair.length();
    let dir = to_crosshair.normalize_or_zero();
    if dir == Vec2::ZERO {
        return;
    }
    let actual_dist = dist.min(DASH_MAX_DISTANCE);
    let target = start + dir * actual_dist;

    // Insère Dashing + Invulnerable simultanément. Le joueur est intouchable
    // pendant toute la fenêtre du dash. `update_dash` retire les deux à la fin.
    commands.entity(player_e).insert((
        Dashing {
            start,
            target,
            elapsed: 0.0,
        },
        Invulnerable,
    ));
    cooldown.timer.reset();
    sfx.play(Sfx::PlayerDash);
}

/// Anime le dash en cours : lerp start → target sur `DASH_DURATION`. À la
/// fin, retire `Dashing` + `Invulnerable` pour rendre le contrôle au joueur.
/// Note : si F1 (debug mode) est actif, `debug_player_invulnerability` ré-
/// insérera `Invulnerable` la frame suivante.
fn update_dash(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut Dashing)>,
) {
    let dt = time.delta_secs();
    for (entity, mut transform, mut dashing) in &mut query {
        dashing.elapsed += dt;
        let t = (dashing.elapsed / DASH_DURATION).clamp(0.0, 1.0);
        let pos = dashing.start.lerp(dashing.target, t);
        transform.translation.x = pos.x;
        transform.translation.y = pos.y;
        if t >= 1.0 {
            if let Ok(mut e) = commands.get_entity(entity) {
                e.remove::<Dashing>();
                e.remove::<Invulnerable>();
            }
        }
    }
}

/// Tick le timer de cooldown chaque frame. Une fois finished, le dash est
/// dispo (`dash_input` autorise un nouveau déclenchement).
fn dash_cooldown_tick(time: Res<Time>, mut cooldown: ResMut<DashCooldown>) {
    cooldown.timer.tick(time.delta());
}

// ─── Dash UI : label "ESPACE" + jauge ───────────────────────────────

fn setup_dash_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/PressStart2P-Regular.ttf");
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(20.0),
                left: Val::Percent(50.0),
                // margin négatif pour centrer (~ demi-largeur du bloc).
                margin: UiRect::left(Val::Px(-DASH_UI_BAR_WIDTH / 2.0)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(4.0),
                ..default()
            },
            DashUI,
        ))
        .with_children(|parent| {
            // Label "ESPACE"
            parent.spawn((
                Text::new("ESPACE"),
                TextFont {
                    font,
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                DashUIText,
            ));
            // Track de la barre (fond sombre)
            parent
                .spawn((
                    Node {
                        width: Val::Px(DASH_UI_BAR_WIDTH),
                        height: Val::Px(DASH_UI_BAR_HEIGHT),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.15, 0.15, 0.15, 0.8)),
                ))
                .with_children(|track| {
                    // Fill (largeur animée par update_dash_ui)
                    track.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(1.0, 0.85, 0.0, 1.0)),
                        DashUIBar,
                    ));
                });
        });
}

/// Met à jour le label (couleur) et la barre (largeur + couleur) selon
/// l'état du cooldown. Dispo = jaune vif, en cooldown = gris + barre cyan
/// qui se remplit.
fn update_dash_ui(
    cooldown: Res<DashCooldown>,
    mut text_q: Query<&mut TextColor, With<DashUIText>>,
    mut bar_q: Query<(&mut Node, &mut BackgroundColor), With<DashUIBar>>,
) {
    let ready = cooldown.timer.is_finished();
    let fraction = cooldown.timer.fraction(); // 0 (vient de claquer) → 1 (dispo)

    if let Ok(mut color) = text_q.single_mut() {
        color.0 = if ready {
            Color::srgba(1.0, 0.85, 0.0, 1.0) // jaune vif
        } else {
            Color::srgba(0.45, 0.45, 0.45, 1.0) // gris
        };
    }
    if let Ok((mut node, mut bg)) = bar_q.single_mut() {
        node.width = Val::Percent(fraction * 100.0);
        bg.0 = if ready {
            Color::srgba(1.0, 0.85, 0.0, 1.0) // jaune (plein)
        } else {
            Color::srgba(0.3, 0.7, 1.0, 1.0) // cyan (en charge)
        };
    }
}

fn cleanup_dash_ui(mut commands: Commands, query: Query<Entity, With<DashUI>>) {
    for entity in query.iter() {
        if let Ok(mut e) = commands.get_entity(entity) {
            e.try_despawn();
        }
    }
}
