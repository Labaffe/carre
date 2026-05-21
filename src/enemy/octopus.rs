//! Octopus — pattern "pendule" déterministe avec double swoop télégraphié.
//!
//! Phase **entering** (intangible) : spawn sans collider, sprite à
//! `ENTERING_OPACITY`. Deux sous-phases :
//! - `entering_rush` (RUSH_IN_DURATION) : `OctopusEnteringRush` + anim `rush`
//!   + `Goto` rectiligne depuis le bord G/D (random) vers le spawn point.
//!   `Added<OctopusEnteringRush>` joue `Sfx::OctopusRush`.
//! - `entering_idle` (IDLE_PAUSE_DURATION) : `OctopusEnteringIdle` + anim `idle`,
//!   immobile à destination. `Added<OctopusEnteringIdle>` joue `Sfx::OctopusSound`.
//! À la fin → état `alive`. `Added<OctopusAlive>` insère le collider et
//! restaure l'alpha à 1.0.
//!
//! Phase **alive** : `OrderedNodeList` qui boucle indéfiniment :
//! 1. **telegraph** (TELEGRAPH_DURATION) : anim `idle`, stationnaire — annonce
//!    le swoop suivant. Donne au joueur le temps de lire le pattern.
//! 2. **swoop** (SWOOP_DURATION) : `OctopusMoving` + anim `rush`.
//!    `octopus_setup_curve` calcule `target = 2·player − start` (miroir
//!    parfait par rapport au joueur) avec un petit jitter en Y. Bézier passe
//!    par `player + perp·SWOOP_BULGE_OFFSET` à t=0.5 (arc visible, joueur
//!    touché sauf déplacement perpendiculaire). Joue `Sfx::OctopusRush`.
//! 3. **telegraph 2** + **swoop 2** : 2e itération du couple précédent. Le
//!    2nd swoop part de la fin du 1er → enchaînement naturel en pendule.
//! 4. **shoot windup** (SHOOT_WINDUP_DURATION) : `OctopusShooting` + anim `shoot`
//!    one-shot. Joue `Sfx::OctopusSound`.
//! 5. **shoot fire** (FIRE_RELEASE_DURATION) : `OctopusFireShots`. Joue
//!    `Sfx::OctopusShoot` + spawn 3 projectiles éventail.
//! 6. Loop (`should_loop`).
//!
//! Mort = `DespawnSelf` immédiat. L'octopus ne peut pas mourir pendant
//! `entering` (pas de collider).

use std::time::Duration;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::audio::{Sfx, SfxPlayer};
use crate::behavior::BehaviorBuilder;
use crate::behavior::behavior::BehaviorComponent;
use crate::behavior::choice_list::TransitionMessages;
use crate::enemy::anim_bank::Animation;
use crate::enemy::death::{DespawnSelf, Dying};
use crate::enemy::enemies::OCTOPUS;
use crate::enemy::enemy::Enemy;
use crate::enemy::enemy_builder::EnemyBuilder;
use crate::game_manager::difficulty::{Difficulty, SpawnPosition};
use crate::geometry::shape::Shape;
use crate::movement::bezier::Bezier;
use crate::movement::bounding_radius::BoundingRadius;
use crate::movement::goto::Goto;
use crate::movement::movement_zone::MovementZone;
use crate::movement::movements::Movements;
use crate::physic::collider::{collider, layers};
use crate::physic::health::Health;
use crate::player::player::Player;
use crate::sprite_orient::FaceMovement;
use crate::weapon::projectile::{ProjectileSpawn, ProjectileSprite, Team, spawn_projectile};

// ─── Constantes ────────────────────────────────────────────────────

/// Durée du télégraphe avant chaque swoop (s) — l'octopus pause en idle
/// pour annoncer le rush imminent. Donne au joueur un signal visuel clair.
const TELEGRAPH_DURATION: f32 = 0.4;
/// Durée d'un swoop (s). Bézier qui traverse le joueur du start au miroir.
/// Plus court que l'ancien MOVE_DURATION pour un rythme plus dynamique.
const SWOOP_DURATION: f32 = 1.8;
/// Durée du wind-up de l'animation `shoot` (s) avant que les projectiles partent.
const SHOOT_WINDUP_DURATION: f32 = 0.6;
/// Durée de la phase "fire release" (s) — court, juste pour laisser le son
/// `OctopusShoot` jouer et marquer la transition.
const FIRE_RELEASE_DURATION: f32 = 0.15;
/// Durée par frame des animations.
const OCTOPUS_FRAME_DURATION: f32 = 0.08;
/// Vitesse des projectiles (px/s). Rapide.
const OCTOPUS_PROJECTILE_SPEED: f32 = 700.0;
/// Hitbox du projectile (rectangle pilule).
const OCTOPUS_PROJECTILE_SIZE: Vec2 = Vec2::new(12.0, 28.0);
/// Demi-angle d'ouverture de l'éventail des 3 tirs (degrés).
const OCTOPUS_SHOT_SPREAD_DEG: f32 = 22.0;
/// Marge intérieure (px) pour le clamp du point cible — évite qu'il colle
/// au bord exact de l'écran.
const TARGET_PICK_MARGIN: f32 = 80.0;
/// Jitter Y (px) ajouté à la position miroir du swoop. Seul élément
/// aléatoire conservé pour éviter le 100% prévisible — l'arrivée varie
/// légèrement en hauteur sans changer le passage par le joueur.
const ARRIVAL_Y_JITTER: f32 = 60.0;
/// Offset perpendiculaire (px) appliqué au pass-through pour donner un arc
/// visible. Direction = règle main droite (CCW) du segment, donc alterne
/// naturellement entre les swoops aller et retour. À 60px, le joueur
/// stationnaire est touché (combined radii ~120), mais peut esquiver en
/// se déplaçant perpendiculairement.
const SWOOP_BULGE_OFFSET: f32 = 60.0;
/// Durée max (s) du rush rectiligne depuis le bord vers le spawn point.
const RUSH_IN_DURATION: f32 = 1.0;
/// Vitesse du rush d'apparition (px/s). Rapide — l'octopus déboule.
const RUSH_IN_SPEED: f32 = 900.0;
/// Petit idle à l'arrivée avant de basculer en alive (s).
const IDLE_PAUSE_DURATION: f32 = 0.5;
/// Offset (px) au-delà du bord pour la position d'entrée. L'octopus part
/// hors écran et glisse vers son spawn.
const ENTRY_OFFSCREEN_OFFSET: f32 = 80.0;
/// Alpha du sprite pendant `entering` (intangible). Indique visuellement
/// au joueur que tirer dessus est inutile.
const ENTERING_OPACITY: f32 = 0.6;
/// Durée totale de l'animation de mort (s). Indépendant du nombre de frames
/// du dossier `images/octopus/death` — la durée par frame est recalculée
/// automatiquement à l'init via `Animation::with_total_duration`.
const OCTOPUS_DEATH_DURATION: f32 = 0.8;

// ─── Composants ────────────────────────────────────────────────────

/// Marker sur l'entité Octopus.
#[derive(Component)]
pub struct Octopus;

/// Posé par la choice pendant la phase de déplacement. Consommé par
/// `octopus_setup_curve` qui insère `Movements` avec une Bézier fraîche.
#[derive(Component, Clone)]
pub struct OctopusMoving;

/// Posé pendant le wind-up de l'animation de tir. `Added<OctopusShooting>`
/// joue le son d'amorçage `OctopusSound`.
#[derive(Component, Clone)]
pub struct OctopusShooting;

/// Posé brièvement à la fin du wind-up. `Added<OctopusFireShots>` joue
/// `OctopusShoot` et spawn les 3 projectiles vers le joueur.
#[derive(Component, Clone)]
pub struct OctopusFireShots;

/// Sous-phase 1 de l'apparition : rush rectiligne depuis le bord G/D vers
/// le spawn point. `Added<OctopusEnteringRush>` joue `Sfx::OctopusRush`.
#[derive(Component, Clone)]
pub struct OctopusEnteringRush;

/// Sous-phase 2 de l'apparition : petit idle à destination.
/// `Added<OctopusEnteringIdle>` joue `Sfx::OctopusSound` (annonce d'arrivée).
#[derive(Component, Clone)]
pub struct OctopusEnteringIdle;

/// Posé quand l'octopus quitte `entering` et devient tangible.
/// `Added<OctopusAlive>` attache le collider et restaure l'alpha à 1.0.
#[derive(Component, Clone)]
pub struct OctopusAlive;

// ─── Builder ───────────────────────────────────────────────────────

pub struct OctopusBuilder {
    timer: Timer,
}

impl OctopusBuilder {
    pub fn new() -> Self {
        Self {
            timer: Timer::new(Duration::ZERO, TimerMode::Once),
        }
    }
}

impl EnemyBuilder for OctopusBuilder {
    fn get_timer(&mut self) -> &mut Timer {
        &mut self.timer
    }
    fn name(&self) -> &'static str {
        "octopus"
    }
    fn preload_anim(&self) -> HashMap<&str, &str> {
        HashMap::from([
            ("octopus_idle", "images/octopus/idle"),
            ("octopus_rush", "images/octopus/rush"),
            ("octopus_shoot", "images/octopus/shoot"),
            ("octopus_death", "images/octopus/death"),
        ])
    }
    fn spawn(
        &self,
        mut commands: Commands,
        window: &Window,
        _difficulty: &ResMut<Difficulty>,
        spawn_pos: SpawnPosition,
        asset_server: &Res<AssetServer>,
    ) {
        let final_pos = spawn_pos.resolve(window, OCTOPUS.config.sprite_size / 2.0);

        // Position d'entrée : hors écran à gauche ou à droite (random), à la
        // même hauteur que le spawn point — la trajectoire est ainsi purement
        // horizontale, plus lisible que diagonale.
        let half_w = window.width() / 2.0;
        let from_right = fastrand::bool();
        let entry_x = if from_right {
            half_w + ENTRY_OFFSCREEN_OFFSET
        } else {
            -(half_w + ENTRY_OFFSCREEN_OFFSET)
        };
        let entry_pos = Vec2::new(entry_x, final_pos.y);

        // ─── Sous-behavior "entering" ─────────────────────────────
        // Phase A : rush rectiligne vers le spawn point.
        // Phase B : petit idle stationnaire à destination.
        // `on_complete` pousse "entering_done" → choice transite en alive.
        let entering = BehaviorBuilder::first(
            Duration::from_secs_f32(RUSH_IN_DURATION),
            BehaviorBuilder::multiple()
                .with(BehaviorBuilder::from_component(OctopusEnteringRush))
                .with(BehaviorBuilder::from_component(Animation::new(
                    "octopus_rush",
                    Duration::from_secs_f32(OCTOPUS_FRAME_DURATION),
                )))
                .with(BehaviorBuilder::from_component(
                    Movements::new().with(Goto::new(final_pos, RUSH_IN_SPEED)),
                )),
        )
        .then(
            Duration::from_secs_f32(IDLE_PAUSE_DURATION),
            BehaviorBuilder::multiple()
                .with(BehaviorBuilder::from_component(OctopusEnteringIdle))
                .with(BehaviorBuilder::from_component(Animation::new(
                    "octopus_idle",
                    Duration::from_secs_f32(OCTOPUS_FRAME_DURATION),
                ))),
        )
        .on_complete("entering_done");

        // ─── Cycle alive : OrderedNodeList bouclée ──────────────
        // 6 phases en séquence : telegraph 1, swoop 1, telegraph 2, swoop 2,
        // shoot windup, fire. `.should_loop()` reprend à 0 après fire.
        let alive_cycle = BehaviorBuilder::first(
            Duration::from_secs_f32(TELEGRAPH_DURATION),
            BehaviorBuilder::from_component(Animation::new(
                "octopus_idle",
                Duration::from_secs_f32(OCTOPUS_FRAME_DURATION),
            )),
        )
        .then(
            Duration::from_secs_f32(SWOOP_DURATION),
            BehaviorBuilder::multiple()
                .with(BehaviorBuilder::from_component(OctopusMoving))
                .with(BehaviorBuilder::from_component(Animation::new(
                    "octopus_rush",
                    Duration::from_secs_f32(OCTOPUS_FRAME_DURATION),
                ))),
        )
        .then(
            Duration::from_secs_f32(TELEGRAPH_DURATION),
            BehaviorBuilder::from_component(Animation::new(
                "octopus_idle",
                Duration::from_secs_f32(OCTOPUS_FRAME_DURATION),
            )),
        )
        .then(
            Duration::from_secs_f32(SWOOP_DURATION),
            BehaviorBuilder::multiple()
                .with(BehaviorBuilder::from_component(OctopusMoving))
                .with(BehaviorBuilder::from_component(Animation::new(
                    "octopus_rush",
                    Duration::from_secs_f32(OCTOPUS_FRAME_DURATION),
                ))),
        )
        .then(
            Duration::from_secs_f32(SHOOT_WINDUP_DURATION),
            BehaviorBuilder::multiple()
                .with(BehaviorBuilder::from_component(OctopusShooting))
                .with(BehaviorBuilder::from_component(
                    Animation::new(
                        "octopus_shoot",
                        Duration::from_secs_f32(OCTOPUS_FRAME_DURATION),
                    )
                    .one_shot(),
                )),
        )
        .then(
            Duration::from_secs_f32(FIRE_RELEASE_DURATION),
            BehaviorBuilder::from_component(OctopusFireShots),
        )
        .should_loop();

        // Wrap dans un multiple avec `OctopusAlive` pour gating du collider.
        // `Added<OctopusAlive>` → `octopus_become_alive` insère le collider
        // et restaure l'alpha à 1.0 (sortie de la phase entering).
        let alive = BehaviorBuilder::multiple()
            .with(BehaviorBuilder::from_component(OctopusAlive))
            .with(alive_cycle);

        // Mort : stoppe le mouvement, joue l'animation de mort one-shot sur
        // OCTOPUS_DEATH_DURATION (la durée par frame est recalculée auto à
        // l'init selon le nombre de frames du dossier). Puis DespawnSelf.
        let dying = BehaviorBuilder::first(
            Duration::from_secs_f32(OCTOPUS_DEATH_DURATION),
            BehaviorBuilder::multiple()
                .with(BehaviorBuilder::from_component(Movements::new()))
                .with(BehaviorBuilder::from_component(
                    Animation::with_total_duration(
                        "octopus_death",
                        Duration::from_secs_f32(OCTOPUS_DEATH_DURATION),
                    )
                    .one_shot(),
                )),
        )
        .then(
            Duration::from_secs_f32(0.1),
            BehaviorBuilder::from_component(DespawnSelf),
        );

        // ─── Outer choice : entering → alive → dying ─────────────
        // Pas de transition "die" depuis entering : sans collider, l'octopus
        // ne peut pas prendre de dégâts pendant cette phase.
        let behavior = BehaviorBuilder::choice()
            .with(entering) // 0
            .with(alive) // 1
            .with(dying) // 2
            .add_transition(0, 1, "entering_done")
            .add_transition(1, 2, "die");

        commands.spawn((
            Sprite {
                image: asset_server.load("images/octopus/frame000.png"),
                custom_size: Some(Vec2::splat(OCTOPUS.config.sprite_size)),
                // Alpha réduit pendant entering — restauré à 1.0 par
                // `octopus_become_alive` à l'entrée du state alive.
                color: Color::srgba(1.0, 1.0, 1.0, ENTERING_OPACITY),
                ..default()
            },
            Transform::from_xyz(entry_pos.x, entry_pos.y, 0.5),
            TransitionMessages::new(),
            Enemy::new(OCTOPUS),
            Health::new(OCTOPUS.total_hp),
            Octopus,
            BoundingRadius(OCTOPUS.config.sprite_size / 2.0),
            // MovementZone pleine écran (margin = 0). Pendant entering la
            // position de départ est hors écran : le clamp ne s'applique pas
            // au start (le tick suivant le `Goto` ramène l'octopus dans la
            // zone, le clamp prend le relais à l'entrée à l'écran).
            MovementZone::new(Vec2::ZERO),
            BehaviorComponent::new(behavior),
            // Pas de `DropTable` → l'octopus ne drop rien à la mort.
            // Sprite naturel orienté à droite → flip quand l'octopus part
            // vers la gauche.
            FaceMovement::faces_right(),
            // PAS de collider ici : l'octopus est intangible pendant entering.
            // `octopus_become_alive` (Added<OctopusAlive>) l'insère ensuite.
        ));
    }
}

// ─── Systèmes réactifs ─────────────────────────────────────────────

/// Joue `OctopusRush` au démarrage de la sous-phase 1 d'entering.
pub fn octopus_entering_rush_sound(
    mut sfx: SfxPlayer,
    query: Query<(), Added<OctopusEnteringRush>>,
) {
    for _ in &query {
        sfx.play(Sfx::OctopusRush);
    }
}

/// Joue `OctopusSound` à l'arrivée (sous-phase 2 d'entering) — l'octopus
/// "s'annonce" avant de devenir agressif.
pub fn octopus_entering_idle_sound(
    mut sfx: SfxPlayer,
    query: Query<(), Added<OctopusEnteringIdle>>,
) {
    for _ in &query {
        sfx.play(Sfx::OctopusSound);
    }
}

/// Joue `OctopusDie` quand le marker `Dying` est inséré sur l'octopus
/// (= HP=0 détecté par `detect_death`). Fire une seule fois par mort.
pub fn octopus_die_sound(
    mut sfx: SfxPlayer,
    query: Query<(), (With<Octopus>, Added<Dying>)>,
) {
    for _ in &query {
        sfx.play(Sfx::OctopusDie);
    }
}

/// `Added<OctopusAlive>` : l'octopus quitte entering, devient tangible.
/// Insère le collider et restaure l'alpha du sprite à 1.0.
pub fn octopus_become_alive(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Sprite), Added<OctopusAlive>>,
) {
    for (entity, mut sprite) in &mut query {
        sprite.color = Color::WHITE;
        if let Ok(mut e) = commands.get_entity(entity) {
            e.insert(collider(
                Shape::Circle(OCTOPUS.config.radius),
                layers::ENEMY,
                layers::PLAYER | layers::PLAYER_PROJECTILE,
            ));
        }
    }
}

/// Sur `Added<OctopusMoving>` : configure le swoop déterministe.
/// `target = 2·player − start` (miroir parfait par rapport au joueur, à un
/// petit jitter Y près), `pass_through = player + perp · SWOOP_BULGE_OFFSET`
/// (offset perpendiculaire au segment, règle main droite). La courbe traverse
/// donc systématiquement la zone du joueur avec un arc visible.
pub fn octopus_setup_curve(
    mut commands: Commands,
    mut sfx: SfxPlayer,
    octopus_q: Query<(Entity, &Transform), (With<Octopus>, Added<OctopusMoving>)>,
    player_q: Query<&Transform, (With<Player>, Without<Octopus>)>,
    windows: Query<&Window>,
) {
    let Ok(window) = windows.single() else { return };
    let half_w = window.physical_width() as f32 / 2.0;
    let half_h = window.physical_height() as f32 / 2.0;
    let target_x_range = (half_w - TARGET_PICK_MARGIN).max(0.0);
    let target_y_range = (half_h - TARGET_PICK_MARGIN).max(0.0);

    let player_pos = player_q
        .single()
        .map(|t| t.translation.truncate())
        .unwrap_or(Vec2::ZERO);

    for (entity, octopus_tf) in &octopus_q {
        let start = octopus_tf.translation.truncate();

        // Target = miroir de start par rapport au joueur, + petit jitter Y
        // pour ne pas être 100% prévisible sur la hauteur d'arrivée.
        let y_jitter = (fastrand::f32() * 2.0 - 1.0) * ARRIVAL_Y_JITTER;
        let mut target = 2.0 * player_pos - start;
        target.y += y_jitter;
        target.x = target.x.clamp(-target_x_range, target_x_range);
        target.y = target.y.clamp(-target_y_range, target_y_range);

        // Pass-through = joueur + offset perpendiculaire au segment (règle
        // main droite). Donne un arc visible et déterministe ; la direction
        // de l'offset alterne naturellement entre swoops aller/retour (le
        // segment change de sens).
        let segment = target - start;
        let segment_len = segment.length().max(1.0);
        let perp = Vec2::new(-segment.y, segment.x) / segment_len;
        let pass_through = player_pos + perp * SWOOP_BULGE_OFFSET;

        if let Ok(mut e) = commands.get_entity(entity) {
            e.insert(Movements::new().with(Bezier::passing_through(
                start,
                pass_through,
                target,
                Duration::from_secs_f32(SWOOP_DURATION),
            )));
        }
        sfx.play(Sfx::OctopusRush);
    }
}

/// Joue le son d'amorçage du tir sur `Added<OctopusShooting>`.
pub fn octopus_shoot_start_sound(mut sfx: SfxPlayer, query: Query<(), Added<OctopusShooting>>) {
    for _ in &query {
        sfx.play(Sfx::OctopusSound);
    }
}

/// Sur `Added<OctopusFireShots>` : joue `OctopusShoot` et spawn 3 projectiles
/// en éventail (-spread / 0 / +spread degrés) vers le joueur.
pub fn octopus_fire_shots(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut sfx: SfxPlayer,
    octopus_q: Query<&Transform, Added<OctopusFireShots>>,
    player_q: Query<&Transform, (With<Player>, Without<Octopus>)>,
) {
    let Ok(player_tf) = player_q.single() else {
        return;
    };
    let player_pos = player_tf.translation.truncate();
    let spread = OCTOPUS_SHOT_SPREAD_DEG.to_radians();
    let angles = [-spread, 0.0, spread];

    for octopus_tf in &octopus_q {
        let origin = octopus_tf.translation;
        let base_dir = (player_pos - origin.truncate()).normalize_or_zero();
        if base_dir == Vec2::ZERO {
            continue;
        }
        for angle in angles {
            let dir = rotate(base_dir, angle);
            spawn_projectile(
                &mut commands,
                &*asset_server,
                ProjectileSpawn {
                    position: Vec3::new(origin.x, origin.y, 0.55),
                    direction: dir,
                    speed: OCTOPUS_PROJECTILE_SPEED,
                    hitbox: Shape::Rect {
                        half_length: OCTOPUS_PROJECTILE_SIZE.y / 2.0,
                        half_width: OCTOPUS_PROJECTILE_SIZE.x / 2.0,
                    },
                    team: Team::Enemy,
                    damage: 1,
                    sprite: ProjectileSprite::Colored {
                        color: Color::srgb(1.0, 0.3, 0.8),
                        size: OCTOPUS_PROJECTILE_SIZE,
                    },
                    death_folder: None,
                },
            );
        }
        sfx.play(Sfx::OctopusShoot);
    }
}

/// Rotation 2D d'un vecteur direction par un angle (radians).
fn rotate(dir: Vec2, angle: f32) -> Vec2 {
    let cos = angle.cos();
    let sin = angle.sin();
    Vec2::new(dir.x * cos - dir.y * sin, dir.x * sin + dir.y * cos)
}
