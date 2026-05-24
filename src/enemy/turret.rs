//! Turret — ennemi statique qui vise le joueur et tire à cadence régulière.
//!
//! Comportement minimal :
//! - **Position** : statique (pas de `Movements`). Si la tourelle est ajoutée
//!   comme enfant d'un groupe en mouvement (futur vaisseau), elle suit
//!   automatiquement via la propagation Bevy `Transform`/`GlobalTransform`.
//! - **Animation** : `turret_fire` (dossier `images/gatling`) en boucle —
//!   les canons tournent en permanence.
//! - **Visée** : à chaque frame, le sprite est tourné pour pointer vers le
//!   joueur (utilise `GlobalTransform` pour fonctionner aussi en child).
//! - **Tir** : `TurretFireTimer` Repeating. À chaque `just_finished`, un
//!   projectile est spawn vers le joueur.
//! - **Mort** : HP=0 → drop éventuel + `DespawnSelf` (via `detect_death` +
//!   `BehaviorComponent` minimal pour les drops).
//!
//! La tourelle ne sort jamais d'elle-même de l'écran — pas de
//! `DespawnOffScreen`. Elle reste tant qu'elle est vivante.

use std::time::Duration;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::audio::{Sfx, SfxPlayer};
use crate::behavior::BehaviorBuilder;
use crate::behavior::behavior::BehaviorComponent;
use crate::behavior::choice_list::TransitionMessages;
use crate::enemy::anim_bank::Animation;
use crate::enemy::death::DespawnSelf;
use crate::enemy::enemies::TURRET;
use crate::enemy::enemy::Enemy;
use crate::enemy::enemy_builder::EnemyBuilder;
use crate::game_manager::difficulty::{Difficulty, SpawnPosition};
use crate::geometry::shape::Shape;
use crate::item::item::{DropTable, ItemType};
use crate::physic::collider::{collider, layers};
use crate::physic::health::Health;
use crate::player::player::Player;
use crate::weapon::projectile::{ProjectileSpawn, ProjectileSprite, Team, spawn_projectile};

// ─── Constantes ────────────────────────────────────────────────────

/// Durée par frame de l'animation des canons (en boucle).
const TURRET_ANIM_FRAME_DURATION: f32 = 0.06;
/// Taille (px) du sprite du module — plus petit que le sprite gatling
/// pour que la base se voie en débordement autour des canons.
const TURRET_MODULE_SIZE: f32 = 140.0;
/// Intervalle entre deux tirs (secondes).
const TURRET_FIRE_INTERVAL: f32 = 0.5;
/// Vitesse des projectiles (px/s).
const TURRET_PROJECTILE_SPEED: f32 = 600.0;
/// Hitbox du projectile (rectangle pilule).
const TURRET_PROJECTILE_SIZE: Vec2 = Vec2::new(8.0, 22.0);
/// Couleur du projectile.
const TURRET_PROJECTILE_COLOR: Color = Color::srgb(1.0, 0.75, 0.2);

static TURRET_DROP_TABLE: [(ItemType, f32); 2] = [
    (ItemType::BonusScore, 0.20),
    (ItemType::Armor, 0.05),
];

// ─── Composants ────────────────────────────────────────────────────

/// Marker sur l'entité Turret.
#[derive(Component)]
pub struct Turret;

/// Timer Repeating qui déclenche un tir à chaque `just_finished`.
/// `Clone` requis pour pouvoir l'injecter via `BehaviorBuilder::from_component`.
#[derive(Component, Clone)]
pub struct TurretFireTimer(pub Timer);

// ─── Builder ───────────────────────────────────────────────────────

pub struct TurretBuilder {
    timer: Timer,
}

impl TurretBuilder {
    pub fn new() -> Self {
        Self {
            timer: Timer::new(Duration::ZERO, TimerMode::Once),
        }
    }
}

impl EnemyBuilder for TurretBuilder {
    fn get_timer(&mut self) -> &mut Timer {
        &mut self.timer
    }
    fn name(&self) -> &'static str {
        "turret"
    }
    fn preload_anim(&self) -> HashMap<&str, &str> {
        HashMap::from([("turret_fire", "images/gatling")])
    }
    fn spawn(
        &self,
        mut commands: Commands,
        window: &Window,
        _difficulty: &ResMut<Difficulty>,
        spawn_pos: SpawnPosition,
        asset_server: &Res<AssetServer>,
    ) {
        let pos = spawn_pos.resolve(window, TURRET.config.sprite_size / 2.0);
        // Module derrière (z plus bas), tourelle devant. Deux entités
        // séparées : la tourelle peut despawn sans emporter le module.
        commands.spawn(turret_module_bundle(asset_server, pos.extend(0.45)));
        commands.spawn(turret_bundle(asset_server, pos.extend(0.5)));
    }
}

/// Module (base) sur lequel la tourelle est posée. **Entité visuelle pure**
/// (Sprite + Transform + `GameplayEntity` pour le cleanup) : pas de
/// collider, pas de Health, pas d'Enemy. Reste à l'écran après la mort de
/// la tourelle posée dessus pour donner l'impression que la base survit.
///
/// Toujours spawn EN MÊME TEMPS que la tourelle, en tant qu'**entité
/// séparée** (sibling, pas child) — sinon la cascade de despawn de la
/// tourelle emporterait aussi le module.
pub fn turret_module_bundle(asset_server: &Res<AssetServer>, position: Vec3) -> impl Bundle {
    (
        Sprite {
            image: asset_server.load("images/turret_module.png"),
            custom_size: Some(Vec2::splat(TURRET_MODULE_SIZE)),
            ..default()
        },
        Transform::from_translation(position),
        // Pas d'Enemy → pas de require(GameplayEntity) implicite. On
        // l'ajoute explicitement pour que `cleanup_playing` despawn le
        // module à la sortie du level.
        crate::GameplayEntity,
    )
}

/// Bundle complet d'une tourelle, paramétré par sa position (relative au
/// parent si la tourelle est spawnée comme enfant). Permet de réutiliser la
/// même définition pour :
/// - une tourelle standalone (`TurretBuilder::spawn`)
/// - une tourelle enfant d'un `EnemyGroup` (cf. `vaisseau.rs`)
pub fn turret_bundle(asset_server: &Res<AssetServer>, position: Vec3) -> impl Bundle {
    // BT minimal : juste un `dying` qui despawn. Présence d'un
    // `BehaviorComponent` + `TransitionMessages` requise pour que
    // `detect_death` fire (et donc émette le `DropEvent`).
    let alive = BehaviorBuilder::multiple()
        .with(BehaviorBuilder::from_component(Animation::new(
            "turret_fire",
            Duration::from_secs_f32(TURRET_ANIM_FRAME_DURATION),
        )))
        .with(BehaviorBuilder::from_component(TurretFireTimer(
            Timer::from_seconds(TURRET_FIRE_INTERVAL, TimerMode::Repeating),
        )));
    let dying = BehaviorBuilder::from_component(DespawnSelf);
    let behavior = BehaviorBuilder::choice()
        .with(alive)
        .with(dying)
        .add_transition(0, 1, "die");

    (
        Sprite {
            image: asset_server.load("images/gatling/frame000.png"),
            custom_size: Some(Vec2::splat(TURRET.config.sprite_size)),
            // Sprite source inversé verticalement : on flip pour que les
            // canons pointent VERS LE HAUT (+Y), cohérent avec la logique
            // de visée de `turret_aim_and_fire` (atan2 - π/2).
            flip_y: true,
            ..default()
        },
        Transform::from_translation(position),
        TransitionMessages::new(),
        Enemy::new(TURRET),
        Health::new(TURRET.total_hp),
        Turret,
        collider(
            Shape::Circle(TURRET.config.radius),
            layers::ENEMY,
            layers::PLAYER | layers::PLAYER_PROJECTILE,
        ),
        BehaviorComponent::new(behavior),
        DropTable {
            drops: &TURRET_DROP_TABLE,
        },
    )
}

// ─── Système : visée + tir ──────────────────────────────────────────

/// Pour chaque tourelle vivante :
/// 1. Calcule la direction vers le joueur (via `GlobalTransform` pour
///    fonctionner correctement même si la tourelle est enfant d'un groupe).
/// 2. Aligne la rotation locale du sprite sur cette direction (sprite source
///    pointe vers le haut → on retire `FRAC_PI_2` pour aligner +Y sur la
///    direction de tir).
/// 3. Tick le `TurretFireTimer` ; à `just_finished`, spawn un projectile
///    qui part dans la direction de visée courante.
pub fn turret_aim_and_fire(
    mut commands: Commands,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    round_sprite: Res<crate::weapon::weapon::RoundSpriteHandle>,
    mut sfx: SfxPlayer,
    player_tf: Single<&GlobalTransform, (With<Player>, Without<Turret>)>,
    mut turret_q: Query<
        (&GlobalTransform, &mut Transform, &mut TurretFireTimer),
        With<Turret>,
    >,
) {
    let player_pos = player_tf.translation().truncate();

    for (gt, mut tf, mut timer) in &mut turret_q {
        let turret_pos = gt.translation().truncate();
        let to_player = player_pos - turret_pos;
        let dir = to_player.normalize_or_zero();
        if dir == Vec2::ZERO {
            continue;
        }

        // Rotation : sprite naturellement orienté vers le haut (+Y).
        // Pour aligner +Y sur `dir`, rotation Z = atan2(dir.y, dir.x) - π/2.
        let angle = dir.y.atan2(dir.x) - std::f32::consts::FRAC_PI_2;
        tf.rotation = Quat::from_rotation_z(angle);

        // Tir périodique.
        timer.0.tick(time.delta());
        if timer.0.just_finished() {
            spawn_projectile(
                &mut commands,
                &*asset_server,
                &round_sprite,
                ProjectileSpawn {
                    position: turret_pos.extend(0.55),
                    direction: dir,
                    speed: TURRET_PROJECTILE_SPEED,
                    hitbox: Shape::Rect {
                        half_length: TURRET_PROJECTILE_SIZE.y / 2.0,
                        half_width: TURRET_PROJECTILE_SIZE.x / 2.0,
                    },
                    team: Team::Enemy,
                    damage: 1,
                    sprite: ProjectileSprite::Colored {
                        color: TURRET_PROJECTILE_COLOR,
                        size: TURRET_PROJECTILE_SIZE,
                    },
                    death_folder: None,
                },
            );
            sfx.play(Sfx::GatlingShoot);
        }
    }
}
