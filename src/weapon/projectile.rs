use crate::movement::despawn_off_screen::DespawnOffScreen;
use crate::game_manager::state::GameState;
use crate::menu::pause::not_paused;
use crate::geometry::shape::Shape;
use crate::physic::collider::{collider, layers};
use bevy::prelude::*;

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            move_projectiles
                .run_if(in_state(GameState::Playing))
                .run_if(not_paused),
        );
    }
}

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
    pub hitbox: Shape,
    pub team: Team,
    pub damage: i32,
    /// Dossier optionnel de frames pour l'animation jouée à l'impact.
    /// Ex: `"images/projectiles/death_missile"`.
    pub death_folder: Option<&'static str>,
}

/// Apparence visuelle d'un projectile.
#[derive(Clone)]
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
    pub hitbox: Shape,
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
    asset_server: &AssetServer,
    spec: ProjectileSpawn,
) -> Entity {
    let dir = spec.direction.normalize_or_zero();
    let velocity = dir.extend(0.0) * spec.speed;

    // Axe local +Y du sprite aligné sur la direction de tir
    let rotation = Quat::from_rotation_z(dir.y.atan2(dir.x) - std::f32::consts::FRAC_PI_2);

    let sprite = match spec.sprite {
        ProjectileSprite::Texture { path, size } => Sprite {
            image: asset_server.load(path),
            custom_size: size,
            ..default()
        },
        ProjectileSprite::Colored { color, size } => Sprite {
            color,
            custom_size: Some(size),
            ..default()
        },
    };

    // Layer + mask selon team. Player projectile détecte ennemis+asteroids,
    // enemy projectile détecte le joueur uniquement.
    let (layer, mask) = match spec.team {
        Team::Player => (
            layers::PLAYER_PROJECTILE,
            layers::ENEMY | layers::ASTEROID,
        ),
        Team::Enemy => (layers::ENEMY_PROJECTILE, layers::PLAYER),
    };
    let proj_collider = collider(spec.hitbox.clone(), layer, mask);

    commands
        .spawn((
            sprite,
            Transform {
                translation: spec.position,
                rotation,
                ..default()
            },
            Projectile {
                velocity,
                hitbox: spec.hitbox,
                team: spec.team,
                damage: spec.damage,
                death_folder: spec.death_folder,
            },
            DespawnOffScreen,
            proj_collider,
        ))
        .id()
}

// ─── Systèmes ───────────────────────────────────────────────────────

/// Déplace tous les projectiles selon leur vélocité.
fn move_projectiles(mut query: Query<(&mut Transform, &Projectile)>, time: Res<Time>) {
    let dt = time.delta_secs();
    for (mut transform, proj) in query.iter_mut() {
        transform.translation += proj.velocity * dt;
    }
}

