//! Tir du joueur : lit l'input, cadence le tir, spawn des `Projectile { Team::Player }`
//! selon la `WeaponDef` équipée, et gère la collision projectile-astéroïde.
//!
//! Le mouvement et le despawn offscreen sont pris en charge par `ProjectilePlugin`.
//! La collision projectile-ennemi est gérée dans `enemy::enemy::projectile_enemy_collision`.

use crate::enemy::asteroid::{Asteroid, HitFlash};
use crate::fx::explosion::{spawn_explosion, spawn_projectile_death};
use crate::game_manager::difficulty::Difficulty;
use crate::game_manager::state::GameState;
use crate::item::item::{DropEvent, DropTable};
use crate::player::player::Player;
use crate::ui::crosshair::Crosshair;
use crate::ui::score::Score;
use crate::weapon::projectile::{
    projectile_hits_circle, spawn_projectile, Projectile, ProjectileSpawn, ProjectileSprite, Team,
};
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
            (shoot, projectile_asteroid_collision).run_if(in_state(GameState::Playing)),
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
    player_q: Query<(&Transform, &Weapon), With<Player>>,
    crosshair_q: Query<&Transform, With<Crosshair>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    if !mouse.pressed(MouseButton::Left) {
        return;
    }

    let Ok((player_transform, weapon)) = player_q.get_single() else {
        return;
    };
    let Ok(crosshair_transform) = crosshair_q.get_single() else {
        return;
    };

    // Adapter la cadence de tir à l'arme actuelle
    fire_timer
        .0
        .set_duration(std::time::Duration::from_secs_f32(weapon.def.fire_rate));
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

    let def = &weapon.def;
    let origin = Vec3::new(player_pos.x, player_pos.y, 0.6); // au-dessus du mothership (0.4)

    // Spawn un projectile par angle dans le pattern
    for shot in def.pattern.iter() {
        let dir = rotate_direction(base_dir, shot.0);

        spawn_projectile(
            &mut commands,
            &asset_server,
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

    commands.spawn(AudioBundle {
        source: asset_server.load("audio/sfx/projectile.ogg"),
        settings: PlaybackSettings::DESPAWN,
    });
}

// ─── Collision projectile joueur → astéroïde ────────────────────────

fn projectile_asteroid_collision(
    mut commands: Commands,
    projectile_q: Query<(Entity, &Transform, &Projectile)>,
    mut asteroid_q: Query<(Entity, &Transform, &mut Asteroid, Option<&DropTable>)>,
    asset_server: Res<AssetServer>,
    mut score: ResMut<Score>,
    difficulty: Res<Difficulty>,
    mut drop_events: EventWriter<DropEvent>,
) {
    let mut despawned_projectiles = std::collections::HashSet::new();
    let mut despawned_asteroids = std::collections::HashSet::new();

    for (projectile_entity, projectile_transform, projectile) in projectile_q.iter() {
        // Seuls les projectiles du joueur touchent les astéroïdes
        if projectile.team != Team::Player {
            continue;
        }
        if despawned_projectiles.contains(&projectile_entity) {
            continue;
        }
        for (asteroid_entity, asteroid_transform, mut asteroid, drop_table) in
            asteroid_q.iter_mut()
        {
            if despawned_asteroids.contains(&asteroid_entity) {
                continue;
            }

            let hit = projectile_hits_circle(
                projectile_transform.translation.truncate(),
                projectile_transform.rotation,
                &projectile.hitbox,
                asteroid_transform.translation.truncate(),
                asteroid.radius,
            );

            if hit {
                spawn_projectile_death(
                    &mut commands,
                    &asset_server,
                    projectile_transform.translation,
                    projectile.death_folder,
                );
                if let Some(mut e) = commands.get_entity(projectile_entity) {
                    e.despawn();
                }
                despawned_projectiles.insert(projectile_entity);
                asteroid.health -= projectile.damage;
                score.add(1);

                if asteroid.health <= 0 {
                    if !despawned_asteroids.contains(&asteroid_entity) {
                        spawn_explosion(
                            &mut commands,
                            &asset_server,
                            asteroid_transform.translation,
                            asteroid.size,
                            asteroid.texture_index,
                            asteroid.base_velocity * difficulty.factor,
                            asteroid_transform.rotation,
                        );
                        commands.spawn(AudioBundle {
                            source: asset_server.load("audio/sfx/asteroid_die.ogg"),
                            settings: PlaybackSettings::DESPAWN,
                        });
                        if let Some(table) = drop_table {
                            drop_events.send(DropEvent {
                                position: asteroid_transform.translation,
                                table: table.drops,
                            });
                        }
                        if let Some(mut e) = commands.get_entity(asteroid_entity) {
                            e.despawn();
                        }
                        despawned_asteroids.insert(asteroid_entity);
                    }
                } else if !despawned_asteroids.contains(&asteroid_entity) {
                    commands
                        .entity(asteroid_entity)
                        .insert(HitFlash(Timer::from_seconds(0.06, TimerMode::Once)));
                    commands.spawn(AudioBundle {
                        source: asset_server.load("audio/sfx/asteroid_hit.ogg"),
                        settings: PlaybackSettings::DESPAWN,
                    });
                }
                break;
            }
        }
    }
}
