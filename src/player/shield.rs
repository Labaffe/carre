//! Pouvoir Bouclier (touche Espace, alternative au dash).
//!
//! Activation : Espace + `ShieldPower` équipé + cooldown prêt → `Shielding`
//! sur le joueur pour `SHIELD_DURATION` secondes. Pendant ce temps :
//! - le joueur reçoit `Invulnerable` (filtre `apply_damage`)
//! - sa hitbox passe de `PLAYER_HITBOX_RADIUS` à `SHIELD_RADIUS`
//! - un cercle bleu translucide apparaît autour de lui
//! - une barre de durée s'affiche au-dessus, qui se vide progressivement
//!
//! La hitbox élargie fait que les projectiles ennemis sont detected et
//! detruits "sur le bouclier" (despawn via `player_damage_on_overlap` qui
//! gère déjà `DESPAWN_ON_PLAYER_HIT` pour ENEMY_PROJECTILE/ASTEROID) — le
//! joueur ne prend pas de dégâts grâce à `Invulnerable`.
//!
//! À l'expiration : retrait de `Invulnerable`, restauration de la hitbox,
//! despawn des visuels (cercle + barre, marqués `ShieldVisual`).
//!
//! Cooldown commence à l'activation (pas à la fin), pour maximiser l'uptime
//! effective. Pendant le cooldown, la barre d'UI dans le bas de l'écran
//! (réutilisée du dash en attendant une UI dédiée) montrerait l'état si
//! le shield était l'arme equipée — pour l'instant l'UI bas reste celle
//! du dash.

use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::sprite_render::{ColorMaterial, MeshMaterial2d};

use crate::audio::{Sfx, SfxPlayer};
use crate::game_manager::state::GameState;
use crate::geometry::shape::Shape;
use crate::menu::pause::not_paused;
use crate::physic::collider::Hitbox;
use crate::physic::invulnerable::Invulnerable;
use crate::player::player::{Player, PLAYER_HITBOX_RADIUS};

// ─── Constantes ────────────────────────────────────────────────────

/// Durée du bouclier (s).
const SHIELD_DURATION: f32 = 2.0;
/// Cooldown entre 2 boucliers (s). Démarre à l'activation.
const SHIELD_COOLDOWN: f32 = 8.0;
/// Rayon du bouclier (px) — taille de la hitbox temporaire + diamètre du
/// cercle visuel. Plus grand que `PLAYER_HITBOX_RADIUS` (45) pour que les
/// projectiles cassent visuellement loin du sprite.
const SHIELD_RADIUS: f32 = 90.0;
/// Couleur du cercle bouclier (RGBA, alpha translucide).
const SHIELD_COLOR: Color = Color::srgba(0.25, 0.55, 1.0, 0.35);
/// Couleur de la barre de durée (fill).
const SHIELD_BAR_FILL: Color = Color::srgba(0.25, 0.8, 1.0, 1.0);
/// Couleur du fond (track) de la barre.
const SHIELD_BAR_TRACK: Color = Color::srgba(0.0, 0.0, 0.0, 0.6);
/// Largeur de la barre de durée (px monde).
const SHIELD_BAR_WIDTH: f32 = 80.0;
/// Hauteur de la barre (px monde).
const SHIELD_BAR_HEIGHT: f32 = 6.0;
/// Offset Y au-dessus du centre joueur pour positionner la barre (px monde).
/// Le sprite joueur fait 128px → demi = 64. +12 = un peu au-dessus.
const SHIELD_BAR_OFFSET_Y: f32 = 76.0;

// ─── Composants & Resource ─────────────────────────────────────────

/// Marker du pouvoir Espace = Bouclier. Le système `shield_input` ne
/// s'exécute que si le joueur porte ce marker. Pour swap depuis Dash :
/// retirer `DashPower`, insérer `ShieldPower`.
#[derive(Component, Default)]
pub struct ShieldPower;

/// Composant actif pendant un bouclier. Tick par `shield_tick` qui met à
/// jour la barre + despawn tout à expiration.
#[derive(Component)]
pub struct Shielding {
    pub timer: Timer,
}

/// Cooldown du bouclier. Identique en API à `DashCooldown` du dash —
/// chaque pouvoir gère son propre cooldown.
#[derive(Resource)]
pub struct ShieldCooldown {
    pub timer: Timer,
}

impl Default for ShieldCooldown {
    fn default() -> Self {
        let mut timer = Timer::from_seconds(SHIELD_COOLDOWN, TimerMode::Once);
        // État initial = prêt.
        timer.tick(std::time::Duration::from_secs_f32(SHIELD_COOLDOWN));
        Self { timer }
    }
}

/// Marker sur les entités visuelles du bouclier (cercle + 2 sprites de la
/// barre), enfants du joueur. Tous despawnés ensemble à l'expiration.
#[derive(Component)]
struct ShieldVisual;

/// Marker sur le sprite de remplissage de la barre — celui dont la largeur
/// est animée par `shield_tick`.
#[derive(Component)]
struct ShieldDurationFill;

/// Marker sur le container UI bas-écran (label "BOUCLIER" + jauge de
/// cooldown). Visibility togglée selon `ShieldPower` équipé.
#[derive(Component)]
struct ShieldUI;
#[derive(Component)]
struct ShieldUIText;
#[derive(Component)]
struct ShieldUIBar;

/// Largeur de la barre UI (px screen).
const SHIELD_UI_BAR_WIDTH: f32 = 120.0;
/// Hauteur de la barre UI (px screen).
const SHIELD_UI_BAR_HEIGHT: f32 = 8.0;

/// Assets partagés (mesh cercle + material bleu) initialisés à `Startup`
/// pour éviter de recréer mesh/material à chaque activation.
#[derive(Resource)]
pub struct ShieldAssets {
    pub circle_mesh: Handle<Mesh>,
    pub circle_material: Handle<ColorMaterial>,
}

// ─── Plugin ─────────────────────────────────────────────────────────

pub struct ShieldPlugin;

impl Plugin for ShieldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShieldCooldown>()
            .add_systems(Startup, setup_shield_assets)
            .add_systems(
                OnEnter(GameState::Playing),
                (reset_shield_cooldown, setup_shield_ui),
            )
            .add_systems(OnExit(GameState::Playing), cleanup_shield_ui)
            .add_systems(
                Update,
                (
                    shield_input,
                    shield_tick,
                    shield_cooldown_tick,
                    update_shield_ui,
                )
                    .run_if(in_state(GameState::Playing))
                    .run_if(not_paused),
            );
    }
}

// ─── Systèmes ──────────────────────────────────────────────────────

fn setup_shield_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.insert_resource(ShieldAssets {
        circle_mesh: meshes.add(Circle::new(1.0)),
        circle_material: materials.add(SHIELD_COLOR),
    });
}

fn reset_shield_cooldown(mut cooldown: ResMut<ShieldCooldown>) {
    *cooldown = ShieldCooldown::default();
}

/// Espace pressé + `ShieldPower` équipé + cooldown prêt + pas déjà
/// `Shielding` → active le bouclier (Invulnerable + hitbox élargie +
/// visuels enfants + reset cooldown).
fn shield_input(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut cooldown: ResMut<ShieldCooldown>,
    mut sfx: SfxPlayer,
    shield_assets: Res<ShieldAssets>,
    player_q: Query<Entity, (With<Player>, With<ShieldPower>, Without<Shielding>)>,
) {
    if !keyboard.just_pressed(KeyCode::Space) {
        return;
    }
    if !cooldown.timer.is_finished() {
        return;
    }
    let Ok(player_e) = player_q.single() else { return };

    // Active le bouclier sur l'entité joueur :
    // - `Shielding` (timer interne pour shield_tick)
    // - `Invulnerable` (filtre apply_damage → pas de dégâts)
    // - `Hitbox` élargie → projectiles cassent "loin" du sprite
    commands.entity(player_e).insert((
        Shielding {
            timer: Timer::from_seconds(SHIELD_DURATION, TimerMode::Once),
        },
        Invulnerable,
        Hitbox(Shape::Circle(SHIELD_RADIUS)),
    ));

    // Spawn les visuels comme enfants du joueur → ils suivent le mouvement
    // automatiquement.
    commands.entity(player_e).with_children(|parent| {
        // Cercle bleu translucide
        parent.spawn((
            Mesh2d(shield_assets.circle_mesh.clone()),
            MeshMaterial2d(shield_assets.circle_material.clone()),
            Transform::from_xyz(0.0, 0.0, 0.55).with_scale(Vec3::splat(SHIELD_RADIUS)),
            ShieldVisual,
        ));
        // Barre de durée — track (fond sombre)
        parent.spawn((
            Sprite {
                color: SHIELD_BAR_TRACK,
                custom_size: Some(Vec2::new(SHIELD_BAR_WIDTH, SHIELD_BAR_HEIGHT)),
                ..default()
            },
            Transform::from_xyz(0.0, SHIELD_BAR_OFFSET_Y, 0.62),
            ShieldVisual,
        ));
        // Barre de durée — fill (animé). `Anchor::CENTER_LEFT` (composant
        // séparé en Bevy 0.18) pour que la barre se vide vers la droite tout
        // en gardant son bord gauche fixe.
        parent.spawn((
            Sprite {
                color: SHIELD_BAR_FILL,
                custom_size: Some(Vec2::new(SHIELD_BAR_WIDTH, SHIELD_BAR_HEIGHT)),
                ..default()
            },
            Anchor::CENTER_LEFT,
            Transform::from_xyz(-SHIELD_BAR_WIDTH / 2.0, SHIELD_BAR_OFFSET_Y, 0.63),
            ShieldVisual,
            ShieldDurationFill,
        ));
    });

    cooldown.timer.reset();
    // Réutilise le son de dash pour l'activation — placeholder, à remplacer
    // par un Sfx::PlayerShield dédié si besoin.
    sfx.play(Sfx::PlayerDash);
}

/// Tick le `Shielding`, met à jour la largeur du fill de la barre, et à
/// expiration : retire `Shielding` + `Invulnerable` + restaure la hitbox
/// normale + despawn tous les visuels (`ShieldVisual`).
fn shield_tick(
    mut commands: Commands,
    time: Res<Time>,
    mut player_q: Query<(Entity, &mut Shielding), With<Player>>,
    mut fill_q: Query<&mut Sprite, With<ShieldDurationFill>>,
    visuals_q: Query<Entity, With<ShieldVisual>>,
) {
    let Ok((player_e, mut shielding)) = player_q.single_mut() else { return };
    shielding.timer.tick(time.delta());

    // Barre qui se vide : largeur = (1 - fraction) * SHIELD_BAR_WIDTH.
    let remaining_ratio = 1.0 - shielding.timer.fraction();
    if let Ok(mut sprite) = fill_q.single_mut() {
        sprite.custom_size = Some(Vec2::new(
            SHIELD_BAR_WIDTH * remaining_ratio,
            SHIELD_BAR_HEIGHT,
        ));
    }

    if shielding.timer.is_finished() {
        // Retire les composants temporaires + restaure la hitbox.
        if let Ok(mut e) = commands.get_entity(player_e) {
            e.remove::<Shielding>();
            e.remove::<Invulnerable>();
            e.insert(Hitbox(Shape::Circle(PLAYER_HITBOX_RADIUS)));
        }
        // Despawn les visuels (cercle + 2 barres) — enfants du joueur mais
        // on les targete par marker pour éviter de toucher d'autres enfants
        // futurs (ex: BoomFlash).
        for entity in visuals_q.iter() {
            if let Ok(mut e) = commands.get_entity(entity) {
                e.try_despawn();
            }
        }
    }
}

fn shield_cooldown_tick(time: Res<Time>, mut cooldown: ResMut<ShieldCooldown>) {
    cooldown.timer.tick(time.delta());
}

// ─── UI bas-écran (label + jauge de cooldown) ──────────────────────

fn setup_shield_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/PressStart2P-Regular.ttf");
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(20.0),
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(-SHIELD_UI_BAR_WIDTH / 2.0)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(4.0),
                ..default()
            },
            // Caché par défaut → `update_shield_ui` toggle Inherited si
            // `ShieldPower` équipé. Évite le flash à l'initialisation.
            Visibility::Hidden,
            ShieldUI,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("BOUCLIER"),
                TextFont {
                    font,
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                ShieldUIText,
            ));
            parent
                .spawn((
                    Node {
                        width: Val::Px(SHIELD_UI_BAR_WIDTH),
                        height: Val::Px(SHIELD_UI_BAR_HEIGHT),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.15, 0.15, 0.15, 0.8)),
                ))
                .with_children(|track| {
                    track.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(SHIELD_BAR_FILL),
                        ShieldUIBar,
                    ));
                });
        });
}

/// Toggle la visibilité du panneau bouclier selon que le joueur a
/// `ShieldPower` équipé. Met à jour le label (couleur) et la jauge de
/// cooldown (largeur + couleur).
fn update_shield_ui(
    cooldown: Res<ShieldCooldown>,
    power_q: Query<(), (With<Player>, With<ShieldPower>)>,
    mut ui_q: Query<&mut Visibility, With<ShieldUI>>,
    mut text_q: Query<&mut TextColor, With<ShieldUIText>>,
    mut bar_q: Query<(&mut Node, &mut BackgroundColor), With<ShieldUIBar>>,
) {
    let equipped = !power_q.is_empty();
    if let Ok(mut vis) = ui_q.single_mut() {
        *vis = if equipped {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    if !equipped {
        return;
    }

    let ready = cooldown.timer.is_finished();
    let fraction = cooldown.timer.fraction();

    if let Ok(mut color) = text_q.single_mut() {
        color.0 = if ready {
            SHIELD_BAR_FILL // cyan vif quand dispo
        } else {
            Color::srgba(0.45, 0.45, 0.45, 1.0)
        };
    }
    if let Ok((mut node, mut bg)) = bar_q.single_mut() {
        node.width = Val::Percent(fraction * 100.0);
        bg.0 = if ready {
            SHIELD_BAR_FILL
        } else {
            Color::srgba(0.2, 0.4, 0.7, 1.0) // bleu plus sombre pendant la charge
        };
    }
}

fn cleanup_shield_ui(mut commands: Commands, q: Query<Entity, With<ShieldUI>>) {
    for entity in q.iter() {
        if let Ok(mut e) = commands.get_entity(entity) {
            e.try_despawn();
        }
    }
}
