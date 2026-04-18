use crate::game_manager::state::GameState;
use crate::menu::pause::not_paused;
use crate::weapon::weapon::HitboxShape;
use bevy::prelude::*;

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (move_projectiles, cleanup_projectiles_offscreen)
                .run_if(in_state(GameState::Playing))
                .run_if(not_paused),
        );
    }
}

// ─── Constantes ─────────────────────────────────────────────────────

/// Limite X (absolue) au-delà de laquelle un projectile est despawné (px).
const OFFSCREEN_X: f32 = 1200.0;
/// Limite Y (absolue) au-delà de laquelle un projectile est despawné (px).
const OFFSCREEN_Y: f32 = 900.0;

// ─── Types ──────────────────────────────────────────────────────────

/// Camp du projectile : détermine quelles entités peuvent être blessées.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Team {
    /// Tiré par le joueur. Peut toucher astéroïdes et ennemis.
    Player,
    /// Tiré par un ennemi. Peut toucher le joueur.
    Enemy,
}

/// Projectile générique. Se déplace en ligne droite à vitesse constante,
/// despawné automatiquement hors écran.
#[derive(Component)]
pub struct Projectile {
    pub velocity: Vec3,
    pub hitbox: HitboxShape,
    pub team: Team,
    pub damage: i32,
    /// Dossier optionnel de frames pour l'animation jouée à l'impact.
    /// Ex: `"images/projectiles/death_missile"`.
    pub death_folder: Option<&'static str>,
}

/// Apparence visuelle d'un projectile.
pub enum ProjectileSprite {
    /// Sprite à partir d'une texture (PNG).
    Texture {
        path: &'static str,
        /// Taille custom (None = taille naturelle de la texture).
        size: Option<Vec2>,
    },
    /// Rectangle coloré (forme pilule si hauteur > largeur).
    Colored { color: Color, size: Vec2 },
}

/// Description complète d'un projectile à spawner.
///
/// Utilisée comme argument de [`spawn_projectile`]. L'entité résultante
/// a sa rotation alignée automatiquement sur la direction de tir.
pub struct ProjectileSpawn {
    /// Position d'origine (monde, z inclus pour le layering).
    pub position: Vec3,
    /// Direction de déplacement. Sera normalisée automatiquement.
    pub direction: Vec2,
    /// Vitesse en pixels/seconde.
    pub speed: f32,
    pub hitbox: HitboxShape,
    pub team: Team,
    pub damage: i32,
    pub sprite: ProjectileSprite,
    pub death_folder: Option<&'static str>,
}

// ─── Spawn ──────────────────────────────────────────────────────────

/// Spawne un projectile et retourne son `Entity`.
///
/// La rotation est alignée automatiquement sur `direction` : l'axe local +Y
/// du sprite pointe dans la direction de déplacement (convention pilule verticale).
pub fn spawn_projectile(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    spec: ProjectileSpawn,
) -> Entity {
    let dir = spec.direction.normalize_or_zero();
    let velocity = dir.extend(0.0) * spec.speed;

    // Axe local +Y du sprite aligné sur la direction de tir
    let rotation = Quat::from_rotation_z(dir.y.atan2(dir.x) - std::f32::consts::FRAC_PI_2);

    let sprite_bundle = match spec.sprite {
        ProjectileSprite::Texture { path, size } => SpriteBundle {
            texture: asset_server.load(path),
            sprite: Sprite {
                custom_size: size,
                ..default()
            },
            transform: Transform {
                translation: spec.position,
                rotation,
                ..default()
            },
            ..default()
        },
        ProjectileSprite::Colored { color, size } => SpriteBundle {
            sprite: Sprite {
                color,
                custom_size: Some(size),
                ..default()
            },
            transform: Transform {
                translation: spec.position,
                rotation,
                ..default()
            },
            ..default()
        },
    };

    commands
        .spawn((
            sprite_bundle,
            Projectile {
                velocity,
                hitbox: spec.hitbox,
                team: spec.team,
                damage: spec.damage,
                death_folder: spec.death_folder,
            },
        ))
        .id()
}

// ─── Collision hitbox vs cercle (helper générique) ─────────────────

/// Test de collision rectangle orienté (OBB) vs cercle.
/// Projette le centre du cercle dans le repère local du rectangle,
/// puis trouve le point le plus proche sur le rectangle.
fn obb_circle_collision(
    rect_pos: Vec2,
    rect_angle: f32,
    half_length: f32,
    half_width: f32,
    circle_pos: Vec2,
    circle_radius: f32,
) -> bool {
    let delta = circle_pos - rect_pos;
    let cos = rect_angle.cos();
    let sin = rect_angle.sin();
    let local_x = delta.dot(Vec2::new(cos, sin));
    let local_y = delta.dot(Vec2::new(-sin, cos));

    let cx = local_x.clamp(-half_width, half_width);
    let cy = local_y.clamp(-half_length, half_length);

    (local_x - cx).powi(2) + (local_y - cy).powi(2) <= circle_radius * circle_radius
}

/// Test de collision entre un projectile (hitbox `Circle` ou `Rect`) et un cercle cible.
///
/// Helper générique utilisé par les pipelines de collision côté joueur et côté ennemi.
pub fn projectile_hits_circle(
    projectile_pos: Vec2,
    projectile_rot: Quat,
    hitbox: &HitboxShape,
    circle_pos: Vec2,
    circle_radius: f32,
) -> bool {
    match hitbox {
        HitboxShape::Circle(r) => projectile_pos.distance(circle_pos) < *r + circle_radius,
        HitboxShape::Rect {
            half_length,
            half_width,
        } => {
            let angle = projectile_rot.to_euler(EulerRot::ZYX).0;
            obb_circle_collision(
                projectile_pos,
                angle,
                *half_length,
                *half_width,
                circle_pos,
                circle_radius,
            )
        }
    }
}

// ─── Systèmes ───────────────────────────────────────────────────────

/// Déplace tous les projectiles selon leur vélocité.
fn move_projectiles(mut query: Query<(&mut Transform, &Projectile)>, time: Res<Time>) {
    let dt = time.delta_seconds();
    for (mut transform, proj) in query.iter_mut() {
        transform.translation += proj.velocity * dt;
    }
}

/// Despawn les projectiles qui sortent des limites d'écran.
fn cleanup_projectiles_offscreen(
    mut commands: Commands,
    query: Query<(Entity, &Transform), With<Projectile>>,
) {
    for (entity, transform) in query.iter() {
        let p = transform.translation;
        if p.x.abs() > OFFSCREEN_X || p.y.abs() > OFFSCREEN_Y {
            if let Some(mut e) = commands.get_entity(entity) {
                e.despawn();
            }
        }
    }
}
