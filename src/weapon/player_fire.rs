//! Tir du joueur : lit l'input, cadence le tir, spawn des `Projectile { Team::Player }`
//! selon la `WeaponDef` équipée.
//!
//! Le mouvement et le despawn offscreen sont pris en charge par `ProjectilePlugin`.
//! Les collisions projectile↔ennemi/astéroïde sont gérées via le pipeline
//! `OverlapEvent` → `projectile_damage_on_overlap` dans `enemy::enemy`.

use crate::audio::{Sfx, SfxPlayer};
use crate::game_manager::state::GameState;
use crate::player::player::{Player, PlayerStats};
use crate::ui::crosshair::Crosshair;
use crate::weapon::projectile::{spawn_projectile, ProjectileSpawn, ProjectileSprite, Team};
use crate::weapon::weapon::Weapon;
use bevy::prelude::*;

pub struct PlayerFirePlugin;

impl Plugin for PlayerFirePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(FireRateTimer(Timer::from_seconds(
            0.2,
            TimerMode::Repeating,
        )))
        .add_systems(OnEnter(GameState::Playing), reset_fire_rate)
        .add_systems(
            Update,
            shoot.run_if(in_state(GameState::Playing)),
        );
    }
}

#[derive(Resource)]
struct FireRateTimer(Timer);

fn reset_fire_rate(mut timer: ResMut<FireRateTimer>) {
    timer.0.reset();
}

// ─── Tir ─────────────────────────────────────────────────────────────

/// Rotation 2D d'un vecteur direction par un angle en radians.
fn rotate_direction(dir: Vec2, angle: f32) -> Vec2 {
    let cos = angle.cos();
    let sin = angle.sin();
    Vec2::new(dir.x * cos - dir.y * sin, dir.x * sin + dir.y * cos)
}

fn shoot(
    mouse: Res<ButtonInput<MouseButton>>,
    mut fire_timer: ResMut<FireRateTimer>,
    time: Res<Time>,
    player_q: Single<(&Transform, &Weapon, &PlayerStats), (With<Player>, Without<crate::player::player::Dashing>)>,
    crosshair_transform: Single<&Transform, With<Crosshair>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut sfx: SfxPlayer,
) {
    if !mouse.pressed(MouseButton::Left) {
        return;
    }

    let (player_transform, weapon, stats) = *player_q;

    let def = weapon.0.def();
    // Adapter la cadence de tir à l'arme actuelle, modulée par les stats
    // (Rapid Fire = fire_rate_mult < 1.0 = tir plus rapide).
    fire_timer
        .0
        .set_duration(std::time::Duration::from_secs_f32(def.fire_rate * stats.fire_rate_mult));
    fire_timer.0.tick(time.delta());
    if !fire_timer.0.just_finished() {
        return;
    }

    let player_pos = player_transform.translation;
    let crosshair_pos = crosshair_transform.translation;
    let base_dir = (crosshair_pos - player_pos).truncate().normalize_or_zero();

    if base_dir == Vec2::ZERO {
        return;
    }

    // `def` réutilisé d'au-dessus (déjà obtenu via weapon.0.def()).
    let origin = Vec3::new(player_pos.x, player_pos.y, 0.6); // au-dessus du mothership (0.4)

    // Spawn un projectile par angle dans le pattern
    for shot in def.pattern.iter() {
        let dir = rotate_direction(base_dir, shot.0);

        spawn_projectile(
            &mut commands,
            &*asset_server,
            ProjectileSpawn {
                position: origin,
                direction: dir,
                speed: def.speed,
                hitbox: def.hitbox.clone(),
                team: Team::Player,
                damage: 1,
                sprite: ProjectileSprite::Texture {
                    path: def.texture_path,
                    size: None,
                },
                death_folder: def.death_folder,
            },
        );
    }

    sfx.play(Sfx::PlayerShoot);
}
