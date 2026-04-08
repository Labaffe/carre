use bevy::prelude::*;

pub struct ExplosionPlugin;

impl Plugin for ExplosionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (animate_explosions, move_explosions));
    }
}

#[derive(Component)]
struct Explosion {
    frames: Vec<Handle<Image>>,
    current_frame: usize,
    timer: Timer,
    velocity: Vec3,
}

/// Nombre max de frames à chercher dans un dossier.
const MAX_DEATH_FRAMES: usize = 32;

// ─── Utilitaire ──────────────────────────────────────────────────────

/// Charge les frames `frame000.png`, `frame001.png`… depuis un dossier.
/// Retourne `None` si `frame000.png` n'existe pas.
fn load_frames_from_folder(
    asset_server: &Res<AssetServer>,
    folder: &str,
) -> Option<Vec<Handle<Image>>> {
    let first = format!("{}/frame000.png", folder);
    let full_path = std::path::Path::new("assets").join(&first);
    if !full_path.exists() {
        return None;
    }

    let mut frames = Vec::new();
    for i in 0..MAX_DEATH_FRAMES {
        let path = format!("{}/frame{:03}.png", folder, i);
        let full = std::path::Path::new("assets").join(&path);
        if !full.exists() {
            break;
        }
        frames.push(asset_server.load(path));
    }

    if frames.is_empty() { None } else { Some(frames) }
}

/// Spawn une animation à une position donnée avec une taille, une vélocité et une durée par frame.
fn spawn_anim(
    commands: &mut Commands,
    frames: Vec<Handle<Image>>,
    position: Vec3,
    size: Vec2,
    velocity: Vec3,
    frame_duration: f32,
) {
    commands.spawn((
        SpriteBundle {
            texture: frames[0].clone(),
            sprite: Sprite {
                custom_size: Some(size),
                ..default()
            },
            transform: Transform::from_translation(position),
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
) {
    let folder = format!("images/asteroids/death_x{:03}", texture_index);
    let frames = load_frames_from_folder(asset_server, &folder)
        .unwrap_or_else(|| load_default_frames(asset_server));

    spawn_anim(commands, frames, position, size, velocity, 0.035);
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
    let Some(folder) = death_folder else { return; };
    let Some(frames) = load_frames_from_folder(asset_server, folder) else { return; };

    spawn_anim(commands, frames, position, Vec2::splat(32.0), Vec3::ZERO, 0.035);
}

// ─── Systèmes ────────────────────────────────────────────────────────

fn move_explosions(
    mut query: Query<(&mut Transform, &Explosion)>,
    time: Res<Time>,
) {
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
