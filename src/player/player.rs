//! Joueur : spawn, mouvement ZQSD, rotation vers le réticule.
//!
//! Vitesse fixe (`PLAYER_SPEED`). La progression du joueur sera ultérieurement
//! pilotée par le deckbuilding (cartes qui modifient vitesse, arme, etc.).
//! L'ancien système de phases temporelles (Phase1/2/3 via timer + boss music)
//! a été retiré.
//!
//! ## Abstraction pour le deckbuilding
//!
//! **Arme (clic gauche)** : le système `shoot` lit `weapon.def` sur l'entité
//! joueur. Pour swap d'arme : muter `weapon.def` (ou réinsérer un nouveau
//! `Weapon`). Pas d'abstraction supplémentaire nécessaire — l'arme courante
//! est déjà data-driven via `WeaponDef`.
//!
//! **Pouvoir (barre Espace)** : pattern **marker-par-pouvoir**. L'entité
//! joueur porte un marker de pouvoir (ex: `DashPower`). Chaque système
//! d'input de pouvoir filtre par son marker dédié. Pour swap de pouvoir :
//! retirer le marker actuel, insérer le nouveau (côté deckbuilding).
//! Seul le pouvoir équipé répond à l'input Espace.
//!
//! Ajouter un nouveau pouvoir = définir son marker + son système d'input
//! filtré + son UI éventuelle, sans toucher aux pouvoirs existants.

use crate::audio::{Sfx, SfxPlayer};
use crate::game_manager::difficulty::BoomEvent;
use crate::game_manager::state::GameState;
use crate::geometry::shape::Shape;
use crate::level::level::{LevelConfig, LevelSetupSet};
use crate::menu::pause::not_paused;
use crate::physic::collider::{collider, layers};
use crate::physic::health::Health;
use crate::physic::invulnerable::Invulnerable;
use crate::player::power::{EquippedPower, PowerCooldowns, PowerKind};
use crate::ui::crosshair::Crosshair;
use crate::weapon::weapon::Weapon;
use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;

// ─── Constantes ────────────────────────────────────────────────────

/// Nombre de vies au départ. `Health` avec ce nombre de PV maximum (un hit = 1 PV).
pub const PLAYER_MAX_LIVES: i32 = 3;
/// Cap d'armure. L'armure absorbe un hit à la place de la vie. Au-delà, les
/// items d'armure ramassés sont consommés pour rien.
pub const PLAYER_MAX_ARMOR: u32 = 3;
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
/// `pub` car le shield s'en sert pour restaurer la hitbox normale après usage.
pub const PLAYER_HITBOX_RADIUS: f32 = 45.0;
/// Durée du flash blanc autour du joueur lors d'un boom.
const BOOM_FLASH_DURATION: f32 = 0.25;
/// Distance maximale d'un dash (px). Si le réticule est plus loin, le dash
/// est plafonné à cette distance dans la direction visée.
const DASH_MAX_DISTANCE: f32 = 350.0;
/// Durée d'un dash (s). Très court — l'effet "blink" qui rend invulnérable
/// (via insert/remove de `Invulnerable` sur la même fenêtre).
/// Cooldown : `PowerKind::Dash.cooldown_seconds()` dans `power.rs`.
const DASH_DURATION: f32 = 0.14;

// ─── Composants ────────────────────────────────────────────────────

#[derive(Component)]
#[require(crate::GameplayEntity)]
pub struct Player;

/// Modificateurs persistants appliqués via les cartes du deckbuilding.
/// Multipliers à 1.0 = stats de base. Reset au respawn (à chaque OnEnter
/// `GameState::Playing`).
#[derive(Component, Debug, Clone, Copy)]
pub struct PlayerStats {
    /// Multiplie `PLAYER_SPEED` dans `movement`.
    pub speed_mult: f32,
    /// Multiplie la durée d'inter-tir (`def.fire_rate`) dans `shoot`. < 1 = plus rapide.
    pub fire_rate_mult: f32,
}

impl Default for PlayerStats {
    fn default() -> Self {
        Self { speed_mult: 1.0, fire_rate_mult: 1.0 }
    }
}

/// Invincibilité temporaire après un hit.
#[derive(Component)]
pub struct Invincible(pub Timer);

/// Marqueur pour le conteneur UI des vies.
#[derive(Component)]
#[require(crate::GameplayEntity)]
pub struct LivesUI;

/// Marqueur individuel pour chaque icône de vie.
#[derive(Component)]
struct LifeIcon(i32);

/// Armure du joueur. Absorbe un hit à la place de la vie tant que `current > 0`.
/// Cf. `apply_damage` dans `physic/health.rs`.
#[derive(Component, Debug)]
pub struct Armor {
    pub current: u32,
    pub max: u32,
}

impl Armor {
    pub fn new(max: u32) -> Self {
        Self { current: 0, max }
    }
}

/// Marqueur pour le conteneur UI de l'armure.
#[derive(Component)]
#[require(crate::GameplayEntity)]
pub struct ArmorUI;

/// Marqueur individuel pour chaque icône d'armure (indexée 0..max).
#[derive(Component)]
struct ArmorIcon(u32);

/// Flash blanc autour du vaisseau lors d'un boom.
#[derive(Component)]
struct BoomFlash(Timer);

/// Composant inséré sur le joueur pendant un dash en cours. Lerp entre
/// `start` et `target` sur `DASH_DURATION`. Tant que ce composant est là :
/// déplacement, tir et rotation sont bloqués (via `Without<Dashing>` dans
/// les queries des systèmes correspondants).
///
/// Le hook `on_remove` retire `Invulnerable` automatiquement quel que soit
/// le moment du retrait (timer expiré, despawn cascade, etc.) — pas de
/// risque d'oubli cleanup.
#[derive(Component)]
#[component(on_remove = dashing_on_remove)]
pub struct Dashing {
    pub start: Vec2,
    pub target: Vec2,
    pub elapsed: f32,
}

/// Hook : retire `Invulnerable` quand `Dashing` est retiré. Comme un seul
/// pouvoir est équipé à la fois (cf. `EquippedPower`), `Invulnerable` ne
/// peut venir que du dash sur le joueur — safe à retirer ici.
/// `DebugInvulnerable` (F1) est un composant séparé, non affecté.
fn dashing_on_remove(mut world: DeferredWorld, ctx: HookContext) {
    world
        .commands()
        .entity(ctx.entity)
        .try_remove::<Invulnerable>();
}

// ─── Plugin ────────────────────────────────────────────────────────

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::Playing),
            (setup_player, setup_lives_ui, setup_armor_ui).after(LevelSetupSet),
        )
        // Cleanup UI : géré centralement par `cleanup_playing` (main.rs)
        // via `#[require(GameplayEntity)]` sur `LivesUI` / `ArmorUI`.
        .add_systems(
            Update,
            (
                movement,
                rotate_towards_crosshair,
                boom_flash_trigger,
                boom_flash_update,
                update_invincibility,
                update_lives_ui,
                update_armor_ui,
                dash_input,
                update_dash,
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
    window: Single<&Window>,
    config: Res<LevelConfig>,
) {
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
        Armor::new(PLAYER_MAX_ARMOR),
        PlayerStats::default(),
        Weapon::default(),
        // Anti-overlap : permet aux Walls (statiques) de repousser le joueur.
        // Pas de NoOverlapStatic → le joueur est mobile, peut donc être poussé.
        crate::movement::bounding_radius::BoundingRadius(PLAYER_HITBOX_RADIUS),
        crate::physic::no_overlap::NoOverlap,
        // Pouvoir Espace équipé. Single source of truth via enum. Swap
        // depuis le deckbuilding = mutation directe de `equipped.0`.
        EquippedPower(PowerKind::Shield),
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
    mut player_q: Single<(&mut Transform, &PlayerStats), (With<Player>, Without<Dashing>)>,
    window: Single<&Window>,
) {
    let half_w = window.width() / 2.0 - PLAYER_MARGIN;
    let half_h = window.height() / 2.0 - PLAYER_MARGIN;

    let mut direction = Vec3::ZERO;

    if keyboard.pressed(KeyCode::KeyW) { direction.y += 1.0; }
    if keyboard.pressed(KeyCode::KeyS) { direction.y -= 1.0; }
    if keyboard.pressed(KeyCode::KeyA) { direction.x -= 1.0; }
    if keyboard.pressed(KeyCode::KeyD) { direction.x += 1.0; }

    let (transform, stats) = &mut *player_q;
    let speed = PLAYER_SPEED * stats.speed_mult;
    transform.translation += direction.normalize_or_zero() * speed * time.delta_secs();
    transform.translation.x = transform.translation.x.clamp(-half_w, half_w);
    transform.translation.y = transform.translation.y.clamp(-half_h, half_h);
}

// ─── Rotation vers le réticule ─────────────────────────────────────

fn rotate_towards_crosshair(
    crosshair_tf: Single<&Transform, (With<Crosshair>, Without<Player>)>,
    mut player_transform: Single<&mut Transform, (With<Player>, Without<Crosshair>, Without<Dashing>)>,
) {
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
            // `try_remove` : safe si l'entité est despawn entre query et flush.
            commands.entity(entity).try_remove::<BoomFlash>();
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
            // `try_remove` : safe si le joueur est despawn entre query et flush
            // (ex: HP=0 le même frame que la fin de l'invincibilité).
            commands.entity(entity).try_remove::<Invincible>();
        } else {
            let blink =
                (inv.0.elapsed_secs() * INVINCIBLE_BLINK_RATE * std::f32::consts::TAU).sin();
            let alpha = if blink > 0.0 { 1.0 } else { 0.0 };
            sprite.color = Color::srgba(1.0, 1.0, 1.0, alpha);
        }
    }
}

// ─── UI des vies ──────────────────────────────────────────────────

/// Couleur appliquée aux icônes "vides" (vie perdue, armure non acquise,
/// 0 bombe) — gris foncé semi-transparent qui contraste bien avec le
/// blanc plein des icônes actives. Partagée avec l'UI bombes (`item.rs`).
pub const HUD_INACTIVE_COLOR: Color = Color::srgba(0.25, 0.25, 0.25, 0.45);

fn setup_lives_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let texture = asset_server.load("images/health.png");

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

/// Au lieu de cacher les icônes perdues (`Visibility::Hidden`), on garde la
/// rangée de `PLAYER_MAX_LIVES` icônes toujours visible et on grise celles
/// au-delà du `Health.current`. Donne un feedback visuel "j'ai perdu une vie"
/// au lieu d'un "il manque un slot".
fn update_lives_ui(
    player_q: Query<&Health, With<Player>>,
    mut icons: Query<(&LifeIcon, &mut ImageNode)>,
) {
    let current_lives = player_q.single().map(|h| h.current).unwrap_or(0);
    for (icon, mut img) in icons.iter_mut() {
        img.color = if icon.0 < current_lives {
            Color::WHITE
        } else {
            HUD_INACTIVE_COLOR
        };
    }
}

// `cleanup_lives_ui` retiré — cleanup auto via `cleanup_playing` (main.rs)
// grâce à `#[require(GameplayEntity)]` sur `LivesUI`.

// ─── UI de l'armure ────────────────────────────────────────────────

/// Spawn la rangée d'icônes d'armure SOUS les vies. Contrairement aux vies
/// (où le slot perdu est grisé pour montrer "j'avais 3, j'ai perdu 1"),
/// l'armure n'affiche QUE les icônes possédées : 0 armure = rien, 3 armures
/// = 3 icônes. Les slots sont créés cachés et révélés par `update_armor_ui`.
fn setup_armor_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let texture = asset_server.load("images/armor.png");

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                // 20 (top vies) + 64 (icône) + 12 (gap vertical) = 96
                top: Val::Px(96.0),
                left: Val::Px(20.0),
                column_gap: Val::Px(12.0),
                ..default()
            },
            ArmorUI,
        ))
        .with_children(|parent| {
            for i in 0..PLAYER_MAX_ARMOR {
                parent.spawn((
                    ImageNode::new(texture.clone()),
                    Node {
                        width: Val::Px(40.0),
                        height: Val::Px(40.0),
                        ..default()
                    },
                    Visibility::Hidden,
                    ArmorIcon(i),
                ));
            }
        });
}

fn update_armor_ui(
    armor: Single<&Armor, With<Player>>,
    mut icons: Query<(&ArmorIcon, &mut Visibility)>,
) {
    let current = armor.current;
    for (icon, mut vis) in icons.iter_mut() {
        *vis = if icon.0 < current {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

// ─── Dash : input, motion ───────────────────────────────────────────

/// Déclenche un dash si :
/// - Espace pressé
/// - `EquippedPower` du joueur == `PowerKind::Dash`
/// - cooldown prêt via `PowerCooldowns`
/// - aucun dash déjà en cours (Without<Dashing> dans la query)
///
/// La cible = position du réticule au moment du clic, plafonnée à
/// `DASH_MAX_DISTANCE` dans la direction visée.
fn dash_input(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut cooldowns: ResMut<PowerCooldowns>,
    mut sfx: SfxPlayer,
    crosshair_tf: Single<&Transform, (With<Crosshair>, Without<Player>)>,
    player_q: Single<
        (Entity, &Transform, &EquippedPower),
        (With<Player>, Without<Dashing>, Without<Crosshair>),
    >,
) {
    if !keyboard.just_pressed(KeyCode::Space) {
        return;
    }
    let (player_e, player_tf, equipped) = *player_q;
    if equipped.0 != PowerKind::Dash {
        return;
    }
    if !cooldowns.is_ready(PowerKind::Dash) {
        return;
    }

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
    cooldowns.trigger(PowerKind::Dash);
    sfx.play(Sfx::PlayerDash);
}

/// Anime le dash en cours : lerp start → target sur `DASH_DURATION`. À la
/// fin, retire `Dashing` — le hook `dashing_on_remove` se charge de retirer
/// `Invulnerable` au passage.
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
            }
        }
    }
}

// Cooldown ticking + UI sont gérés centralement par `PowerPlugin`
// (cf. `src/player/power.rs`). Le dash n'a plus besoin de ses propres
// systèmes pour ça.
