//! Système d'animations de mort (astéroïdes et projectiles).
//!
//! - Astéroïdes : cherche d'abord un dossier custom `death_xNNN/` avec des frames.
//!   S'il existe → animation personnalisée. Sinon → explosion générique (4 frames).
//! - Projectiles : si `death_folder` est défini dans la WeaponDef et contient des frames,
//!   l'animation est jouée. Sinon le projectile disparaît sans effet.
//! - Convention de nommage des frames : `frame000.png`, `frame001.png`, etc.
//! - Toutes les animations durent exactement DEATH_ANIM_DURATION secondes,
//!   la durée par frame est calculée automatiquement.
//! - L'animation conserve la vélocité et la rotation de l'entité d'origine.

use bevy::prelude::*;

pub struct ExplosionPlugin;

impl Plugin for ExplosionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (animate_explosions, move_explosions));
    }
}

#[derive(Component)]
pub struct Explosion {
    frames: Vec<Handle<Image>>,
    current_frame: usize,
    timer: Timer,
    velocity: Vec3,
}

// ─── Utilitaire ──────────────────────────────────────────────────────

/// Scanne un dossier pour trouver tous les fichiers `frameNNN.png`,
/// les trie par index croissant, et retourne les handles.
/// Supporte les trous dans la numérotation (ex: frame000, frame004, frame008).
/// Retourne `None` si le dossier n'existe pas ou ne contient aucune frame.
fn load_frames_from_folder(
    asset_server: &Res<AssetServer>,
    folder: &str,
) -> Option<Vec<Handle<Image>>> {
    let dir_path = std::path::Path::new("assets").join(folder);
    let read_dir = std::fs::read_dir(&dir_path).ok()?;

    let mut entries: Vec<(usize, String)> = Vec::new();

    for entry in read_dir.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy().to_string();
        // Cherche les fichiers frameNNN.png (peu importe le nombre de chiffres)
        if let Some(rest) = name.strip_prefix("frame") {
            if let Some(num_str) = rest.strip_suffix(".png") {
                if let Ok(index) = num_str.parse::<usize>() {
                    entries.push((index, name));
                }
            }
        }
    }

    if entries.is_empty() {
        return None;
    }

    // Tri par index croissant pour jouer les frames dans l'ordre
    entries.sort_by_key(|(i, _)| *i);

    let frames = entries
        .into_iter()
        .map(|(_, name)| asset_server.load(format!("{}/{}", folder, name)))
        .collect();

    Some(frames)
}

/// Durée totale fixe d'une animation de mort (en secondes).
const DEATH_ANIM_DURATION: f32 = 0.25;

/// Spawn une animation à une position donnée.
/// La durée par frame est calculée pour que l'animation totale dure toujours `DEATH_ANIM_DURATION`.
fn spawn_anim(
    commands: &mut Commands,
    frames: Vec<Handle<Image>>,
    position: Vec3,
    size: Vec2,
    velocity: Vec3,
    rotation: Quat,
) {
    let frame_duration = DEATH_ANIM_DURATION / frames.len() as f32;
    commands.spawn((
        SpriteBundle {
            texture: frames[0].clone(),
            sprite: Sprite {
                custom_size: Some(size),
                ..default()
            },
            transform: Transform {
                translation: position,
                rotation,
                ..default()
            },
            ..default()
        },
        Explosion {
            frames,
            current_frame: 0,
            timer: Timer::from_seconds(frame_duration, TimerMode::Repeating),
            velocity,
        },
    ));
}

// ─── Explosion astéroïde ─────────────────────────────────────────────

/// Charge les frames par défaut (explosion générique).
fn load_default_frames(asset_server: &Res<AssetServer>) -> Vec<Handle<Image>> {
    (1..=4)
        .map(|i| asset_server.load(format!("images/explosion/explosion_{}.png", i)))
        .collect()
}

/// Spawn une explosion pour un astéroïde.
/// `velocity` : vélocité de l'astéroïde au moment de sa mort (conservée par l'animation).
pub fn spawn_explosion(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    position: Vec3,
    size: Vec2,
    texture_index: usize,
    velocity: Vec3,
    rotation: Quat,
) {
    let folder = format!("images/asteroids/death_x{:03}", texture_index);
    let frames = load_frames_from_folder(asset_server, &folder)
        .unwrap_or_else(|| load_default_frames(asset_server));

    spawn_anim(commands, frames, position, size, velocity, rotation);
}

// ─── Mort projectile ─────────────────────────────────────────────────

/// Spawn une animation de mort pour un projectile.
/// Si `death_folder` contient des frames, l'animation est jouée.
/// Sinon, le projectile disparaît sans effet visuel.
pub fn spawn_projectile_death(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    position: Vec3,
    death_folder: Option<&str>,
) {
    let Some(folder) = death_folder else {
        return;
    };
    let Some(frames) = load_frames_from_folder(asset_server, folder) else {
        return;
    };

    spawn_anim(
        commands,
        frames,
        position,
        Vec2::splat(32.0),
        Vec3::ZERO,
        Quat::IDENTITY,
    );
}

// ─── Systèmes ────────────────────────────────────────────────────────

fn move_explosions(mut query: Query<(&mut Transform, &Explosion)>, time: Res<Time>) {
    for (mut transform, explosion) in query.iter_mut() {
        transform.translation += explosion.velocity * time.delta_seconds();
    }
}

fn animate_explosions(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Handle<Image>, &mut Explosion)>,
) {
    for (entity, mut texture, mut explosion) in query.iter_mut() {
        explosion.timer.tick(time.delta());

        if explosion.timer.just_finished() {
            explosion.current_frame += 1;
            if explosion.current_frame >= explosion.frames.len() {
                commands.entity(entity).despawn();
            } else {
                *texture = explosion.frames[explosion.current_frame].clone();
            }
        }
    }
}
