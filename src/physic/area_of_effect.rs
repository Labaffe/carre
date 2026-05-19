//! Zone de dégâts générique (Area Of Effect).
//!
//! Une entité avec un [`AreaOfEffect`] est un `Hittable` qui touche le joueur
//! au contact mais ne despawn pas (`despawn_on_hit = false`) — elle persiste
//! pour sa `lifetime`. Le joueur prend 1 dégât puis devient `Invincible` pour
//! `INVINCIBLE_DURATION` (=2s), donc en pratique une AOE de lifetime ≤ 2s
//! n'inflige qu'un seul hit.
//!
//! **Visuel** : utilise des `Mesh2d` partagés (cercle unité + quad unité)
//! stockés dans [`AoeAssets`]. Le scale du `Transform` adapte la taille.
//! La forme de collision (`Shape`) est respectée visuellement : Circle → vrai
//! cercle, Rect → rectangle.
//!
//! Spawn via [`spawn_aoe`] (signature avec assets) ou manuellement.

use bevy::prelude::*;
use bevy::sprite_render::{ColorMaterial, MeshMaterial2d};

use crate::geometry::shape::Shape;
use crate::physic::collision::Hittable;

#[derive(Component)]
pub struct AreaOfEffect {
    pub shape: Shape,
    pub lifetime: f32,
    pub elapsed: f32,
}

impl Hittable for AreaOfEffect {
    fn hitbox_shape(&self) -> Shape {
        self.shape.clone()
    }
    /// L'AOE persiste pour toute sa lifetime — le joueur peut entrer/sortir
    /// sans la consommer.
    fn despawn_on_hit(&self) -> bool {
        false
    }
}

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
        ))
        .id()
}
