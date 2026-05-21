//! Zone de dégâts générique (Area Of Effect).
//!
//! Une entité avec un [`AreaOfEffect`] porte un collider de layer `AOE` qui
//! émet des `OverlapEvent` pour player/ennemis/asteroids. Le joueur prend 1
//! dégât (via `player_damage_on_overlap`) puis devient `Invincible` pour
//! `INVINCIBLE_DURATION` (=2s), donc une AOE de lifetime ≤ 2s n'inflige
//! qu'un seul hit. Les ennemis prennent `AOE_DAMAGE_TO_ENEMY` une fois
//! (via `aoe_damage_enemies_on_overlap` + tracking `AoeAlreadyHit`).
//! L'AOE persiste pour sa `lifetime` puis se despawn (`aoe_lifecycle`).
//!
//! **Visuel** — deux modes :
//! 1. [`spawn_aoe`] : `Mesh2d` partagé (cercle unité ou quad unité) coloré
//!    rouge translucide, scaled par le `Transform`. Cercle/Rect respecté.
//! 2. [`spawn_aoe_animated`] : `Sprite` avec une animation jouée en one-shot
//!    sur toute la `lifetime` de l'AOE (durée par frame recalculée auto).
//!    L'animation doit être préchargée dans [`AnimBank`] via le `preload_anim`
//!    de l'ennemi qui spawn l'AOE.

use std::time::Duration;

use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy::sprite_render::{ColorMaterial, MeshMaterial2d};

use crate::enemy::anim_bank::{Animation, AnimBank};
use crate::geometry::shape::Shape;
use crate::physic::collider::{collider, layers, OverlapEvent};
use crate::physic::health::DamageEvent;

#[derive(Component)]
#[require(crate::GameplayEntity)]
pub struct AreaOfEffect {
    pub shape: Shape,
    pub lifetime: f32,
    pub elapsed: f32,
}

/// Dégâts infligés par une AOE à un ennemi/astéroïde dans sa zone. Une seule
/// fois par couple (AOE, target) — pas de damage tick frame-par-frame.
pub const AOE_DAMAGE_TO_ENEMY: i32 = 50;

/// Tracking des entités déjà touchées par cette AOE — évite le damage tick
/// répété tant que l'entité reste dans la zone.
#[derive(Component, Default)]
pub struct AoeAlreadyHit(pub HashSet<Entity>);

/// Assets partagés pour le rendu des AOE : un cercle unité et un quad unité
/// (réutilisables, le `Transform.scale` adapte les dimensions). Évite de
/// créer un Mesh+Material par AOE.
#[derive(Resource)]
pub struct AoeAssets {
    pub circle_mesh: Handle<Mesh>,
    pub quad_mesh: Handle<Mesh>,
    pub material: Handle<ColorMaterial>,
}

pub fn setup_aoe_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.insert_resource(AoeAssets {
        // Cercle unité (rayon 1) — scaler via Transform.scale = Vec3::splat(rayon)
        circle_mesh: meshes.add(Circle::new(1.0)),
        // Quad unité (1×1) — scaler via Transform.scale = (width, height, 1)
        quad_mesh: meshes.add(Rectangle::new(1.0, 1.0)),
        material: materials.add(Color::srgba(1.0, 0.0, 0.0, 0.3)),
    });
}

/// Tick la lifetime de chaque AOE et despawn quand expirée.
pub fn aoe_lifecycle(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut AreaOfEffect)>,
) {
    let dt = time.delta_secs();
    for (entity, mut aoe) in &mut query {
        aoe.elapsed += dt;
        if aoe.elapsed >= aoe.lifetime {
            commands.entity(entity).try_despawn();
        }
    }
}

/// Helper de spawn : crée une AOE avec un visuel par défaut (mesh rouge
/// translucide selon la `shape`). Nécessite `AoeAssets` en paramètre — typique
/// via `Res<AoeAssets>` côté caller.
pub fn spawn_aoe(
    commands: &mut Commands,
    aoe_assets: &AoeAssets,
    position: Vec3,
    shape: Shape,
    lifetime: f32,
) -> Entity {
    let (mesh, scale) = match &shape {
        Shape::Circle(r) => (aoe_assets.circle_mesh.clone(), Vec3::splat(*r)),
        Shape::Rect {
            half_length,
            half_width,
        } => (
            aoe_assets.quad_mesh.clone(),
            Vec3::new(*half_width * 2.0, *half_length * 2.0, 1.0),
        ),
    };
    let shape_clone = shape.clone();
    commands
        .spawn((
            Mesh2d(mesh),
            MeshMaterial2d(aoe_assets.material.clone()),
            Transform::from_translation(position).with_scale(scale),
            AreaOfEffect {
                shape,
                lifetime,
                elapsed: 0.0,
            },
            AoeAlreadyHit::default(),
            collider(
                shape_clone,
                layers::AOE,
                layers::PLAYER | layers::ENEMY | layers::ASTEROID,
            ),
        ))
        .id()
}

/// Helper de spawn : crée une AOE avec un `Sprite` animé à la place du mesh.
/// L'animation est jouée en one-shot sur toute la `lifetime` (la durée par
/// frame est calculée automatiquement : `lifetime / frame_count`).
///
/// `anim_name` doit avoir été préchargé dans `AnimBank` via le `preload_anim`
/// d'un `EnemyBuilder`. La 1ère frame est récupérée depuis l'`AnimBank`
/// directement pour éviter un flash blanc le temps que le système `animate`
/// se déclenche à la 1ère tick. `sprite_size` est la taille (px, monde) du
/// sprite — indépendante de la `shape` du collider.
pub fn spawn_aoe_animated(
    commands: &mut Commands,
    anim_bank: &AnimBank,
    position: Vec3,
    shape: Shape,
    lifetime: f32,
    anim_name: &'static str,
    sprite_size: f32,
) -> Entity {
    let shape_clone = shape.clone();
    let initial_image = anim_bank
        .get(&anim_name.to_string())
        .and_then(|frames| frames.first().cloned())
        .unwrap_or_default();
    commands
        .spawn((
            Sprite {
                image: initial_image,
                custom_size: Some(Vec2::splat(sprite_size)),
                ..default()
            },
            Animation::with_total_duration(anim_name, Duration::from_secs_f32(lifetime))
                .one_shot(),
            Transform::from_translation(position),
            AreaOfEffect {
                shape,
                lifetime,
                elapsed: 0.0,
            },
            AoeAlreadyHit::default(),
            collider(
                shape_clone,
                layers::AOE,
                layers::PLAYER | layers::ENEMY | layers::ASTEROID,
            ),
        ))
        .id()
}

/// Réactif sur `OverlapEvent` : pour chaque overlap AOE↔ENEMY/ASTEROID,
/// émet un `DamageEvent` une seule fois par couple (AOE, target).
/// L'AOE elle-même n'est pas despawnée — elle continue jusqu'à sa lifetime.
pub fn aoe_damage_enemies_on_overlap(
    mut events: MessageReader<OverlapEvent>,
    mut aoe_q: Query<&mut AoeAlreadyHit>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for ev in events.read() {
        let Some((aoe_e, target_e)) = ev.pick(layers::AOE) else { continue };
        let target_layer = if ev.a == target_e { ev.a_layer } else { ev.b_layer };
        if target_layer & (layers::ENEMY | layers::ASTEROID) == 0 {
            continue;
        }
        let Ok(mut already_hit) = aoe_q.get_mut(aoe_e) else { continue };
        // insert retourne true si pas déjà présent
        if !already_hit.0.insert(target_e) {
            continue;
        }
        damage_events.write(DamageEvent {
            target: target_e,
            amount: AOE_DAMAGE_TO_ENEMY,
            source: Some(aoe_e),
        });
    }
}
