//! Pouvoir Bouclier (touche Espace).
//!
//! Activation : Espace + `EquippedPower == PowerKind::Shield` + cooldown
//! prêt → insère `Shielding` + `Invulnerable` + hitbox élargie + visuels
//! (cercle bleu + barre de durée au-dessus du joueur).
//!
//! Pendant la fenêtre : le joueur reçoit aucun dégât (`Invulnerable` →
//! `apply_damage` skip). La hitbox élargie fait que les projectiles ennemis
//! sont détruits "sur le bouclier" (despawn via `player_damage_on_overlap`
//! qui gère déjà `DESPAWN_ON_PLAYER_HIT` pour `ENEMY_PROJECTILE | ASTEROID`).
//!
//! À l'expiration : retrait de `Invulnerable`, restauration de la hitbox,
//! despawn des visuels marqués `ShieldVisual`.
//!
//! Le cooldown + l'UI bas-écran sont gérés centralement par `PowerPlugin`
//! (cf. `power.rs`). Le `ShieldPlugin` ne s'occupe que de l'activation et
//! des effets visuels propres au shield.

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
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
use crate::player::power::{EquippedPower, PowerCooldowns, PowerKind};

// ─── Constantes ────────────────────────────────────────────────────

/// Durée du bouclier (s). Cooldown : `PowerKind::Shield.cooldown_seconds()`.
const SHIELD_DURATION: f32 = 2.0;
/// Rayon du bouclier (px) — taille de la hitbox temporaire + diamètre du
/// cercle visuel.
const SHIELD_RADIUS: f32 = 90.0;
/// Couleur du cercle bouclier (RGBA, alpha translucide).
const SHIELD_COLOR: Color = Color::srgba(0.25, 0.55, 1.0, 0.35);
/// Couleur de la barre de durée au-dessus du joueur (fill).
const SHIELD_BAR_FILL: Color = Color::srgba(0.25, 0.8, 1.0, 1.0);
/// Couleur du fond (track) de la barre.
const SHIELD_BAR_TRACK: Color = Color::srgba(0.0, 0.0, 0.0, 0.6);
const SHIELD_BAR_WIDTH: f32 = 80.0;
const SHIELD_BAR_HEIGHT: f32 = 6.0;
const SHIELD_BAR_OFFSET_Y: f32 = 76.0;

// ─── Composants ────────────────────────────────────────────────────

/// Composant actif pendant un bouclier. Tick par `shield_tick` qui met à
/// jour la barre + retire le composant à expiration. **Tout le cleanup**
/// (Invulnerable, restauration hitbox, despawn des visuels) est délégué
/// au hook `on_remove` — peu importe la raison du retrait (timer fini,
/// despawn joueur, swap manuel), le state revient propre.
#[derive(Component)]
#[component(on_remove = shielding_on_remove)]
pub struct Shielding {
    pub timer: Timer,
}

/// Hook déclenché quand `Shielding` est retiré de l'entité (timer expiré,
/// despawn cascade, swap, etc.). Restaure la hitbox normale, retire
/// `Invulnerable`, et despawn les enfants marqués `ShieldVisual` du joueur.
fn shielding_on_remove(mut world: DeferredWorld, ctx: HookContext) {
    let entity = ctx.entity;

    // Restauration directe via mutation (DeferredWorld permet get_mut).
    if let Some(mut hitbox) = world.get_mut::<Hitbox>(entity) {
        *hitbox = Hitbox(Shape::Circle(PLAYER_HITBOX_RADIUS));
    }

    // Snapshot des enfants avant d'invoquer commands (release la borrow).
    let children: Vec<Entity> = world
        .get::<Children>(entity)
        .map(|c| c.iter().collect())
        .unwrap_or_default();
    let visual_children: Vec<Entity> = children
        .into_iter()
        .filter(|&child| world.get::<ShieldVisual>(child).is_some())
        .collect();

    // Queue les mutations différées (commands appliquées au prochain sync).
    let mut commands = world.commands();
    commands.entity(entity).try_remove::<Invulnerable>();
    for child in visual_children {
        commands.entity(child).try_despawn();
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
        app.add_systems(Startup, setup_shield_assets)
            .add_systems(
                Update,
                (shield_input, shield_tick)
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

/// Active le bouclier si Espace pressé + Shield équipé + cooldown prêt +
/// pas déjà actif. Insère Shielding/Invulnerable/Hitbox élargie + spawn
/// les visuels enfants du joueur.
fn shield_input(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut cooldowns: ResMut<PowerCooldowns>,
    mut sfx: SfxPlayer,
    shield_assets: Res<ShieldAssets>,
    player_q: Query<(Entity, &EquippedPower), (With<Player>, Without<Shielding>)>,
) {
    if !keyboard.just_pressed(KeyCode::Space) {
        return;
    }
    let Ok((player_e, equipped)) = player_q.single() else { return };
    if equipped.0 != PowerKind::Shield {
        return;
    }
    if !cooldowns.is_ready(PowerKind::Shield) {
        return;
    }

    commands.entity(player_e).insert((
        Shielding {
            timer: Timer::from_seconds(SHIELD_DURATION, TimerMode::Once),
        },
        Invulnerable,
        Hitbox(Shape::Circle(SHIELD_RADIUS)),
    ));

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

    cooldowns.trigger(PowerKind::Shield);
    // Réutilise le son de dash en placeholder.
    sfx.play(Sfx::PlayerDash);
}

/// Tick le `Shielding`, met à jour la largeur du fill de la barre. À
/// expiration : `remove::<Shielding>` — le hook `shielding_on_remove`
/// se charge de tout le cleanup (hitbox, Invulnerable, visuels).
fn shield_tick(
    mut commands: Commands,
    time: Res<Time>,
    mut player_q: Query<(Entity, &mut Shielding), With<Player>>,
    mut fill_q: Query<&mut Sprite, With<ShieldDurationFill>>,
) {
    let Ok((player_e, mut shielding)) = player_q.single_mut() else { return };
    shielding.timer.tick(time.delta());

    let remaining_ratio = 1.0 - shielding.timer.fraction();
    if let Ok(mut sprite) = fill_q.single_mut() {
        sprite.custom_size = Some(Vec2::new(
            SHIELD_BAR_WIDTH * remaining_ratio,
            SHIELD_BAR_HEIGHT,
        ));
    }

    if shielding.timer.is_finished() {
        if let Ok(mut e) = commands.get_entity(player_e) {
            e.remove::<Shielding>();
        }
    }
}
