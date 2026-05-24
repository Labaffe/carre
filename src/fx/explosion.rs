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
        app.init_resource::<ExplosionFramesCache>()
            .add_systems(Update, (animate_explosions, move_explosions));
    }
}

/// Cache de frames d'explosion par dossier. Évite un `fs::read_dir` par
/// spawn d'explosion (hot path : chaque mort d'ennemi/projectile).
/// Le premier appel pour un dossier scanne le disque ; les suivants
/// retournent les handles déjà chargés (Bevy gère le cache des handles
/// natif, mais c'est le `read_dir` qui était coûteux).
#[derive(Resource, Default)]
pub struct ExplosionFramesCache {
    by_folder: bevy::platform::collections::HashMap<String, Vec<Handle<Image>>>,
}

impl ExplosionFramesCache {
    pub fn get_or_load(
        &mut self,
        asset_server: &AssetServer,
        folder: &str,
    ) -> Option<Vec<Handle<Image>>> {
        if let Some(f) = self.by_folder.get(folder) {
            return Some(f.clone());
        }
        let frames = load_frames_from_folder_uncached(asset_server, folder)?;
        self.by_folder.insert(folder.to_string(), frames.clone());
        Some(frames)
    }
}

#[derive(Component)]
#[require(crate::GameplayEntity)]
pub struct Explosion {
    frames: Vec<Handle<Image>>,
    current_frame: usize,
    timer: Timer,
    velocity: Vec3,
}

// ─── Utilitaire ──────────────────────────────────────────────────────

/// Variante interne sans cache : scanne le disque (fs::read_dir + load).
/// Utiliser `ExplosionFramesCache::get_or_load` dans le hot path.
fn load_frames_from_folder_uncached(
    asset_server: &AssetServer,
    folder: &str,
) -> Option<Vec<Handle<Image>>> {
    let dir_path = std::path::Path::new("assets").join(folder);
    let read_dir = std::fs::read_dir(&dir_path).ok()?;

    let mut entries: Vec<(usize, String)> = Vec::new();

    for entry in read_dir.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy().to_string();
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

    entries.sort_by_key(|(i, _)| *i);

    let frames = entries
        .into_iter()
        .map(|(_, name)| asset_server.load(format!("{}/{}", folder, name)))
        .collect();

    Some(frames)
}

/// Scanne un dossier (fs::read_dir + load) pour trouver les `frameNNN.png`.
/// **Pas adapté au hot path** : appelle `ExplosionFramesCache::get_or_load`
/// si tu spawn des explosions à la volée. Conservé public pour les
/// préchargements one-shot (cf. `preload_item_frames`).
pub fn load_frames_from_folder(
    asset_server: &Res<AssetServer>,
    folder: &str,
) -> Option<Vec<Handle<Image>>> {
    load_frames_from_folder_uncached(asset_server, folder)
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
        Sprite { image: frames[0].clone(), custom_size: Some(size), ..default() },
        Transform { translation: position, rotation, ..default() },
        Explosion {
            frames,
            current_frame: 0,
            timer: Timer::from_seconds(frame_duration, TimerMode::Repeating),
            velocity,
        },
    ));
}

/// Spawn une animation à partir de frames préchargées avec une durée custom.
/// Durée totale = `duration` secondes, la durée par frame est calculée.
pub fn spawn_custom_anim(
    commands: &mut Commands,
    frames: Vec<Handle<Image>>,
    position: Vec3,
    size: Vec2,
    duration: f32,
) {
    let frame_duration = duration / frames.len() as f32;
    commands.spawn((
        (Sprite { image: frames[0].clone(), custom_size: Some(size), ..default() }, Transform::from_translation(position)),
        Explosion {
            frames,
            current_frame: 0,
            timer: Timer::from_seconds(frame_duration, TimerMode::Repeating),
            velocity: Vec3::ZERO,
        },
    ));
}

// ─── Explosion astéroïde ─────────────────────────────────────────────

/// Charge les frames par défaut (explosion générique).
fn load_default_frames(asset_server: &AssetServer) -> Vec<Handle<Image>> {
    (1..=4)
        .map(|i| asset_server.load(format!("images/explosion/explosion_{}.png", i)))
        .collect()
}

/// Spawn une explosion pour un astéroïde, en passant par le cache pour
/// éviter un `fs::read_dir` par mort.
pub fn spawn_explosion(
    commands: &mut Commands,
    asset_server: &AssetServer,
    cache: &mut ExplosionFramesCache,
    position: Vec3,
    size: Vec2,
    texture_index: usize,
    velocity: Vec3,
    rotation: Quat,
) {
    let folder = format!("images/asteroids/death_x{:03}", texture_index);
    let frames = cache
        .get_or_load(asset_server, &folder)
        .unwrap_or_else(|| load_default_frames(asset_server));

    spawn_anim(commands, frames, position, size, velocity, rotation);
}

// ─── Mort projectile ─────────────────────────────────────────────────

/// Spawn une animation de mort pour un projectile, via le cache.
pub fn spawn_projectile_death(
    commands: &mut Commands,
    asset_server: &AssetServer,
    cache: &mut ExplosionFramesCache,
    position: Vec3,
    death_folder: Option<&str>,
) {
    let Some(folder) = death_folder else {
        return;
    };
    let Some(frames) = cache.get_or_load(asset_server, folder) else {
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
        transform.translation += explosion.velocity * time.delta_secs();
    }
}

fn animate_explosions(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Sprite, &mut Explosion)>,
) {
    for (entity, mut sprite, mut explosion) in query.iter_mut() {
        explosion.timer.tick(time.delta());

        if explosion.timer.just_finished() {
            explosion.current_frame += 1;
            if explosion.current_frame >= explosion.frames.len() {
                if let Ok(mut e) = commands.get_entity(entity) { e.try_despawn(); }
            } else {
                sprite.image = explosion.frames[explosion.current_frame].clone();
            }
        }
    }
}
