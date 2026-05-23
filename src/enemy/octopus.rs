//! Octopus — telegraph + action aléatoire (swoop ou shoot) à chaque cycle.
//!
//! Phase **entering** (intangible) : spawn sans collider, sprite à
//! `OCTOPUS_INTANGIBLE_TINT`. Une seule sous-phase :
//! - `entering_rush` (RUSH_IN_DURATION) : `OctopusEnteringRush` + anim `rush`
//!   + `Goto` rectiligne depuis le bord G/D (random) vers le spawn point.
//!   `Added<OctopusEnteringRush>` joue `Sfx::OctopusRush`.
//! Dès que le `Goto` se termine → état `alive`. `Added<OctopusAlive>` insère
//! le collider et restaure l'alpha à 1.0.
//!
//! Phase **alive** : `ChoiceNodeList` à 3 états cyclant indéfiniment :
//! 1. **telegraph** : `OctopusTelegraph` + anim `idle`, stationnaire. Le
//!    système `octopus_telegraph_tick` tick son timer interne ; à
//!    `TELEGRAPH_DURATION` il pousse aléatoirement `"do_swoop"` ou
//!    `"do_shoot"` (50/50) dans `TransitionMessages`.
//! 2. **swoop** (SWOOP_DURATION, sur `"do_swoop"`) : `OctopusMoving` + anim
//!    `rush`. `octopus_setup_curve` calcule `target = 2·player − start`
//!    (miroir + jitter Y). Bézier passe par `player + perp·SWOOP_BULGE_OFFSET`
//!    à t=0.5. Joue `Sfx::OctopusRush`. `on_complete("back_to_telegraph")`.
//! 3. **shoot** (SHOOT_WINDUP_DURATION + FIRE_RELEASE_DURATION, sur `"do_shoot"`) :
//!    `OctopusShooting` + anim `shoot` one-shot → `OctopusFireShots`. Sons
//!    `OctopusSound` puis `OctopusShoot` + spawn 3 projectiles éventail.
//!    `on_complete("back_to_telegraph")`.
//!
//! Transitions choice :
//! - 0 → 1 sur `"do_swoop"`
//! - 0 → 2 sur `"do_shoot"`
//! - 1 → 0 sur `"back_to_telegraph"`
//! - 2 → 0 sur `"back_to_telegraph"`
//!
//! Mort = anim octopus_death one-shot puis `DespawnSelf`. L'octopus ne peut
//! pas mourir pendant `entering` (pas de collider).

use std::time::Duration;

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::audio::{Sfx, SfxPlayer, spawn_sfx};
use crate::behavior::BehaviorBuilder;
use crate::behavior::behavior::BehaviorComponent;
use crate::behavior::choice_list::TransitionMessages;
use crate::enemy::anim_bank::Animation;
use crate::enemy::death::{DespawnSelf, Dying};
use crate::enemy::enemies::{EnemyData, OCTOPUS, OCTOPUS_GREEN};
use crate::enemy::enemy::Enemy;
use crate::enemy::enemy_builder::EnemyBuilder;
use crate::game_manager::difficulty::{Difficulty, SpawnPosition};
use crate::geometry::shape::Shape;
use crate::movement::bezier::Bezier;
use crate::movement::bounding_radius::BoundingRadius;
use crate::movement::goto::Goto;
use crate::movement::movement_zone::MovementZone;
use crate::movement::movements::Movements;
use crate::physic::area_of_effect::spawn_aoe_animated;
use crate::physic::collider::{collider, layers, CollidesWith, CollisionLayer, Hitbox};
use crate::physic::health::Health;
use crate::player::player::Player;
use crate::sprite_orient::FaceMovement;
use crate::weapon::projectile::{ProjectileSpawn, ProjectileSprite, Team, spawn_projectile};

// ─── Constantes ────────────────────────────────────────────────────

/// Durée du télégraphe avant chaque swoop (s) — l'octopus pause en idle
/// pour annoncer le rush imminent. Donne au joueur un signal visuel clair.
const TELEGRAPH_DURATION: f32 = 0.4;
/// Durée d'un swoop (s). Bézier qui traverse le joueur du start au miroir.
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
/// Variante verte : nombre de projectiles tirés en éventail.
const OCTOPUS_GREEN_SHOT_COUNT: usize = 4;
/// Variante verte : demi-angle de l'éventail (plus large car 4 tirs).
const OCTOPUS_GREEN_SHOT_SPREAD_DEG: f32 = 32.0;
/// Variante verte : couleur des projectiles (vert vif).
const OCTOPUS_GREEN_PROJECTILE_COLOR: Color = Color::srgb(0.2, 1.0, 0.4);
/// Teinte appliquée au sprite quand l'octopus est **intangible** — utilisée
/// dans deux contextes : la phase `entering` (apparition, base + vert), et
/// le swoop du vert. Multiplier RGB < 1.0 → assombrit nettement le sprite,
/// signal visuel clair "tu ne peux pas me toucher".
const OCTOPUS_INTANGIBLE_TINT: Color = Color::srgba(0.35, 0.35, 0.35, 0.9);

/// Variante verte : pendant le swoop, l'octopus jette des bombes en cloche
/// (trajectoire Bézier). Une nouvelle bombe est lancée tous les
/// `OCTOPUS_GREEN_BOMB_INTERVAL` secondes ; elle explose en AOE quand le
/// timer `OCTOPUS_GREEN_BOMB_FLIGHT_DURATION` arrive à terme (= moment où
/// elle "tombe" au point d'impact calculé).
const OCTOPUS_GREEN_BOMB_INTERVAL: f32 = 0.4;
const OCTOPUS_GREEN_BOMB_FLIGHT_DURATION: f32 = 1.0;
/// Hauteur (px) de l'apex de l'arc parabolique au-dessus du segment
/// start→landing. Plus c'est grand, plus la cloche est haute.
const OCTOPUS_GREEN_BOMB_ARC_HEIGHT: f32 = 180.0;
const OCTOPUS_GREEN_BOMB_AOE_RADIUS: f32 = 70.0;
const OCTOPUS_GREEN_BOMB_AOE_LIFETIME: f32 = 0.4;
const OCTOPUS_GREEN_BOMB_AOE_SPRITE_SIZE: f32 = 140.0;
const OCTOPUS_GREEN_BOMB_SPRITE_SIZE: f32 = 48.0;
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
/// Offset (px) au-delà du bord pour la position d'entrée. L'octopus part
/// hors écran et glisse vers son spawn.
const ENTRY_OFFSCREEN_OFFSET: f32 = 80.0;
// Note : la teinte d'intangibilité (entering ET swoop vert) est définie
// par `OCTOPUS_INTANGIBLE_TINT` plus haut — pas de constante alpha séparée.
/// Durée totale de l'animation de mort (s). Indépendant du nombre de frames
/// du dossier `images/octopus/death` — la durée par frame est recalculée
/// automatiquement à l'init via `Animation::with_total_duration`.
const OCTOPUS_DEATH_DURATION: f32 = 0.8;
/// Durée du télégraphe avant un swoop (s). Trois `Sfx::OctopusRush` sont
/// joués sur cette durée pour annoncer l'attaque ; le 3e coïncide avec le
/// début effectif du swoop (joué par `octopus_setup_curve`).
const PRE_SWOOP_DURATION: f32 = 0.6;
/// Nombre de sons joués DANS la phase pre-swoop. Le 3e son total vient du
/// début de la phase swoop (via `octopus_setup_curve`).
const PRE_SWOOP_SOUNDS_IN_PHASE: u8 = 1;

// ─── Composants ────────────────────────────────────────────────────

/// Marker sur l'entité Octopus.
#[derive(Component)]
pub struct Octopus;

/// Marker additionnel sur les octopus de la variante verte. Posé EN PLUS
/// de `Octopus` (donc tous les systèmes `With<Octopus>` continuent de
/// tourner pour le vert) ; permet de différencier le comportement spécifique
/// via des filtres `With<OctopusGreen>` / `Without<OctopusGreen>`.
///
/// Différences vert vs base :
/// - Swoop : cible aléatoire (vs miroir-joueur), intangible pendant le rush.
/// - Swoop : largue des bombes vertes en cloche (cf. `OctopusGreenBombThrower`).
/// - Tir : 4 projectiles verts (vs 3 roses).
#[derive(Component, Clone)]
pub struct OctopusGreen;

/// Posé sur un octopus vert pendant la phase de swoop. Le timer tick chaque
/// frame ; à chaque `just_finished` une bombe verte est jetée en cloche
/// (`octopus_green_throw_bombs`). Inséré par `octopus_green_setup_curve`,
/// retiré par `octopus_green_swoop_end`.
#[derive(Component)]
pub struct OctopusGreenBombThrower {
    pub timer: Timer,
}

/// Bombe verte en vol parabolique. Le `lifetime` correspond exactement à
/// la durée de l'arc Bézier ; à expiration l'entité despawn et une AOE est
/// spawnée à la position courante (= point d'impact).
#[derive(Component)]
#[require(crate::GameplayEntity)]
pub struct OctopusGreenBomb {
    pub lifetime: Timer,
}

/// Posé par la choice pendant la phase de déplacement. Consommé par
/// `octopus_setup_curve` qui insère `Movements` avec une Bézier fraîche.
/// Le hook `on_insert` joue `Sfx::OctopusRush`.
#[derive(Component, Clone)]
#[component(on_insert = play_octopus_rush)]
pub struct OctopusMoving;

/// Posé pendant le wind-up de l'animation de tir. Le hook `on_insert`
/// joue le son d'amorçage `OctopusSound`.
#[derive(Component, Clone)]
#[component(on_insert = play_octopus_sound)]
pub struct OctopusShooting;

/// Posé brièvement à la fin du wind-up. Le hook `on_insert` joue
/// `OctopusShoot` ; `octopus_fire_shots` spawn les 3 projectiles.
#[derive(Component, Clone)]
#[component(on_insert = play_octopus_shoot)]
pub struct OctopusFireShots;

/// Sous-phase 1 de l'apparition : rush rectiligne depuis le bord G/D vers
/// le spawn point. Le hook `on_insert` joue `Sfx::OctopusRush`.
#[derive(Component, Clone)]
#[component(on_insert = play_octopus_rush)]
pub struct OctopusEnteringRush;

/// Posé quand l'octopus quitte `entering` et devient tangible.
/// `Added<OctopusAlive>` attache le collider et restaure l'alpha à 1.0.
#[derive(Component, Clone)]
pub struct OctopusAlive;

/// Timer interne de l'état telegraph. Ticked par `octopus_telegraph_tick`,
/// qui pousse aléatoirement `"do_swoop"` ou `"do_shoot"` dans
/// `TransitionMessages` quand `elapsed >= TELEGRAPH_DURATION`. `fired` gate
/// pour ne pas re-pousser à chaque frame une fois écoulé.
#[derive(Component, Clone, Default)]
pub struct OctopusTelegraph {
    pub elapsed: f32,
    pub fired: bool,
}

/// Posé pendant la phase pre-swoop (idle télégraphe de 1.2s avant chaque
/// rush). Ticked par `octopus_pre_swoop_tick` qui joue 2 `Sfx::OctopusRush`
/// espacés (le 3e son vient du début de la phase swoop via
/// `octopus_setup_curve`). `sounds_played` gate pour ne pas spammer.
#[derive(Component, Clone, Default)]
pub struct OctopusPreSwoop {
    pub elapsed: f32,
    pub sounds_played: u8,
}

// ─── Variantes (octopus standard + octopus vert) ───────────────────

/// Set de noms d'animations + sprite de base d'une variante d'octopus.
/// Les builders standard / vert n'en diffèrent QUE par ces 5 strings.
struct OctopusAnims {
    idle: &'static str,
    rush: &'static str,
    shoot: &'static str,
    death: &'static str,
    base_sprite: &'static str,
}

const ANIMS_STD: OctopusAnims = OctopusAnims {
    idle: "octopus_idle",
    rush: "octopus_rush",
    shoot: "octopus_shoot",
    death: "octopus_death",
    base_sprite: "images/octopus/frame000.png",
};

const ANIMS_GREEN: OctopusAnims = OctopusAnims {
    idle: "octopus_green_idle",
    rush: "octopus_green_rush",
    shoot: "octopus_green_shoot",
    death: "octopus_green_death",
    base_sprite: "images/octopus_green/frame000.png",
};

/// Construit l'arbre de comportement complet de l'octopus (entering → alive
/// cycle → dying) pour une variante donnée. La structure est identique pour
/// le standard et le vert ; seuls les noms d'anims diffèrent.
fn build_octopus_behavior(
    final_pos: Vec2,
    anims: &OctopusAnims,
) -> impl crate::behavior::behavior::Behavior + Send + Sync + 'static {
    // ─── Entering : rush vers le spawn point puis bascule alive ──
    // Pas de pause idle d'arrivée : dès que le Goto se termine, le BT
    // pousse "entering_done" → le cycle alive démarre directement.
    let entering = BehaviorBuilder::first(
        Duration::from_secs_f32(RUSH_IN_DURATION),
        BehaviorBuilder::multiple()
            .with(BehaviorBuilder::from_component(OctopusEnteringRush))
            .with(BehaviorBuilder::from_component(Animation::new(
                anims.rush,
                Duration::from_secs_f32(OCTOPUS_FRAME_DURATION),
            )))
            .with(BehaviorBuilder::from_component(
                Movements::new().with(Goto::new(final_pos, RUSH_IN_SPEED)),
            )),
    )
    .on_complete("entering_done");

    // ─── Cycle alive : choice à 3 états ─────────────────────
    let telegraph = BehaviorBuilder::multiple()
        .with(BehaviorBuilder::from_component(OctopusTelegraph::default()))
        .with(BehaviorBuilder::from_component(Animation::new(
            anims.idle,
            Duration::from_secs_f32(OCTOPUS_FRAME_DURATION),
        )));

    let swoop = BehaviorBuilder::first(
        Duration::from_secs_f32(PRE_SWOOP_DURATION),
        BehaviorBuilder::multiple()
            .with(BehaviorBuilder::from_component(OctopusPreSwoop::default()))
            .with(BehaviorBuilder::from_component(Animation::new(
                anims.idle,
                Duration::from_secs_f32(OCTOPUS_FRAME_DURATION),
            ))),
    )
    .then(
        Duration::from_secs_f32(SWOOP_DURATION),
        BehaviorBuilder::multiple()
            .with(BehaviorBuilder::from_component(OctopusMoving))
            .with(BehaviorBuilder::from_component(Animation::new(
                anims.rush,
                Duration::from_secs_f32(OCTOPUS_FRAME_DURATION),
            ))),
    )
    .on_complete("back_to_telegraph");

    let shoot = BehaviorBuilder::first(
        Duration::from_secs_f32(SHOOT_WINDUP_DURATION),
        BehaviorBuilder::multiple()
            .with(BehaviorBuilder::from_component(OctopusShooting))
            .with(BehaviorBuilder::from_component(
                Animation::new(
                    anims.shoot,
                    Duration::from_secs_f32(OCTOPUS_FRAME_DURATION),
                )
                .one_shot(),
            )),
    )
    .then(
        Duration::from_secs_f32(FIRE_RELEASE_DURATION),
        BehaviorBuilder::from_component(OctopusFireShots),
    )
    .on_complete("back_to_telegraph");

    let alive_cycle = BehaviorBuilder::choice()
        .with(telegraph) // 0
        .with(swoop) // 1
        .with(shoot) // 2
        .add_transition(0, 1, "do_swoop")
        .add_transition(0, 2, "do_shoot")
        .add_transition(1, 0, "back_to_telegraph")
        .add_transition(2, 0, "back_to_telegraph");

    let alive = BehaviorBuilder::multiple()
        .with(BehaviorBuilder::from_component(OctopusAlive))
        .with(alive_cycle);

    let dying = BehaviorBuilder::first(
        Duration::from_secs_f32(OCTOPUS_DEATH_DURATION),
        BehaviorBuilder::multiple()
            .with(BehaviorBuilder::from_component(Movements::new()))
            .with(BehaviorBuilder::from_component(
                Animation::with_total_duration(
                    anims.death,
                    Duration::from_secs_f32(OCTOPUS_DEATH_DURATION),
                )
                .one_shot(),
            )),
    )
    .then(
        Duration::from_secs_f32(0.1),
        BehaviorBuilder::from_component(DespawnSelf),
    );

    BehaviorBuilder::choice()
        .with(entering) // 0
        .with(alive) // 1
        .with(dying) // 2
        .add_transition(0, 1, "entering_done")
        .add_transition(1, 2, "die")
}

/// Spawn un octopus de la variante donnée à `spawn_pos`. `extra_markers`
/// permet aux variantes d'ajouter leurs propres composants (ex: `OctopusGreen`
/// pour la variante verte) en plus du marker `Octopus` commun.
fn spawn_octopus_variant<B: Bundle>(
    commands: &mut Commands,
    window: &Window,
    spawn_pos: SpawnPosition,
    asset_server: &Res<AssetServer>,
    data: EnemyData,
    anims: &OctopusAnims,
    extra_markers: B,
) {
    let final_pos = spawn_pos.resolve(window, data.config.sprite_size / 2.0);

    // Position d'entrée : hors écran à gauche ou à droite (random), à la
    // même hauteur que le spawn point — trajectoire horizontale lisible.
    let half_w = window.width() / 2.0;
    let from_right = fastrand::bool();
    let entry_x = if from_right {
        half_w + ENTRY_OFFSCREEN_OFFSET
    } else {
        -(half_w + ENTRY_OFFSCREEN_OFFSET)
    };
    let entry_pos = Vec2::new(entry_x, final_pos.y);

    let behavior = build_octopus_behavior(final_pos, anims);

    commands.spawn((
        Sprite {
            image: asset_server.load(anims.base_sprite),
            custom_size: Some(Vec2::splat(data.config.sprite_size)),
            // Teinte sombre pendant entering — signal d'intangibilité.
            // Restauré à `Color::WHITE` par `octopus_become_alive` à
            // l'entrée du state alive.
            color: OCTOPUS_INTANGIBLE_TINT,
            ..default()
        },
        Transform::from_xyz(entry_pos.x, entry_pos.y, 0.5),
        TransitionMessages::new(),
        Enemy::new(data),
        Health::new(data.total_hp),
        Octopus,
        BoundingRadius(data.config.sprite_size / 2.0),
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
        extra_markers,
        // PAS de collider ici : l'octopus est intangible pendant entering.
        // `octopus_become_alive` (Added<OctopusAlive>) l'insère ensuite.
    ));
}

// ─── Builder : octopus standard ───────────────────────────────────

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
        spawn_octopus_variant(
            &mut commands,
            window,
            spawn_pos,
            asset_server,
            OCTOPUS,
            &ANIMS_STD,
            (),
        );
    }
}

// ─── Builder : octopus vert ────────────────────────────────────────

pub struct OctopusGreenBuilder {
    timer: Timer,
}

impl OctopusGreenBuilder {
    pub fn new() -> Self {
        Self {
            timer: Timer::new(Duration::ZERO, TimerMode::Once),
        }
    }
}

impl EnemyBuilder for OctopusGreenBuilder {
    fn get_timer(&mut self) -> &mut Timer {
        &mut self.timer
    }
    fn name(&self) -> &'static str {
        "octopus_green"
    }
    fn preload_anim(&self) -> HashMap<&str, &str> {
        HashMap::from([
            ("octopus_green_idle", "images/octopus_green/idle"),
            ("octopus_green_rush", "images/octopus_green/rush"),
            ("octopus_green_shoot", "images/octopus_green/shoot"),
            ("octopus_green_death", "images/octopus_green/death"),
            // AOE des bombes vertes — variante hue-shiftée de
            // `images/mine/explosion` générée via `tools/sprite_hue_shift.py`.
            ("octopus_green_explosion", "images/octopus_green/explosion"),
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
        spawn_octopus_variant(
            &mut commands,
            window,
            spawn_pos,
            asset_server,
            OCTOPUS_GREEN,
            &ANIMS_GREEN,
            OctopusGreen,
        );
    }
}

// ─── Hooks audio (cf. attributs `#[component(on_insert = ...)]`) ──

/// Hook commun joué à l'insertion de `OctopusMoving` et `OctopusEnteringRush`.
fn play_octopus_rush(mut world: DeferredWorld, _: HookContext) {
    spawn_sfx(&mut world, Sfx::OctopusRush);
}

/// Hook joué à l'insertion de `OctopusShooting`.
fn play_octopus_sound(mut world: DeferredWorld, _: HookContext) {
    spawn_sfx(&mut world, Sfx::OctopusSound);
}

/// Hook joué à l'insertion de `OctopusFireShots`.
fn play_octopus_shoot(mut world: DeferredWorld, _: HookContext) {
    spawn_sfx(&mut world, Sfx::OctopusShoot);
}

// ─── Systèmes réactifs ─────────────────────────────────────────────

/// Joue `OctopusDie` quand le marker `Dying` est inséré sur l'octopus
/// (= HP=0 détecté par `detect_death`). Fire une seule fois par mort.
/// **Reste un système** car `Dying` est partagé entre tous les ennemis :
/// on ne peut pas mettre un hook sur `Dying` qui ne fire que pour Octopus.
pub fn octopus_die_sound(mut sfx: SfxPlayer, query: Query<(), (With<Octopus>, Added<Dying>)>) {
    for _ in &query {
        sfx.play(Sfx::OctopusDie);
    }
}

/// Tick chaque `OctopusPreSwoop` actif et joue `Sfx::OctopusRush` aux
/// thresholds : t=0, t=PRE_SWOOP_DURATION/2 (= 0.6s à 1.2s total). Le 3e
/// son du télégraphe est joué au début de la phase swoop par
/// `octopus_setup_curve` (via `Added<OctopusMoving>`), donnant un trio
/// régulier 0 / 0.6 / 1.2 où le 3e coïncide avec le début du rush.
pub fn octopus_pre_swoop_tick(
    time: Res<Time>,
    mut sfx: SfxPlayer,
    mut query: Query<&mut OctopusPreSwoop>,
) {
    let dt = time.delta_secs();
    let interval = PRE_SWOOP_DURATION / PRE_SWOOP_SOUNDS_IN_PHASE as f32;
    for mut state in &mut query {
        state.elapsed += dt;
        while state.sounds_played < PRE_SWOOP_SOUNDS_IN_PHASE {
            let threshold = state.sounds_played as f32 * interval;
            if state.elapsed >= threshold {
                sfx.play(Sfx::OctopusRush);
                state.sounds_played += 1;
            } else {
                break;
            }
        }
    }
}

/// Tick le timer interne de chaque `OctopusTelegraph` actif. Quand le timer
/// dépasse `TELEGRAPH_DURATION`, pousse aléatoirement `"do_swoop"` ou
/// `"do_shoot"` (50/50) dans `TransitionMessages` pour que la choice
/// transitionne vers l'action correspondante. Le flag `fired` garantit
/// qu'un seul message est poussé par instance de telegraph.
pub fn octopus_telegraph_tick(
    time: Res<Time>,
    mut query: Query<(&mut TransitionMessages, &mut OctopusTelegraph)>,
) {
    let dt = time.delta_secs();
    for (mut msgs, mut telegraph) in &mut query {
        if telegraph.fired {
            continue;
        }
        telegraph.elapsed += dt;
        if telegraph.elapsed >= TELEGRAPH_DURATION {
            let action = if fastrand::bool() {
                "do_swoop"
            } else {
                "do_shoot"
            };
            msgs.messages.push(action.to_string());
            telegraph.fired = true;
        }
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
            // `try_insert` : safe si despawn entre `get_entity` et le flush.
            e.try_insert(collider(
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
    octopus_q: Query<
        (Entity, &Transform),
        (With<Octopus>, Without<OctopusGreen>, Added<OctopusMoving>),
    >,
    player_q: Query<&Transform, (With<Player>, Without<Octopus>)>,
    window: Single<&Window>,
) {
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
            e.try_insert(Movements::new().with(Bezier::passing_through(
                start,
                pass_through,
                target,
                Duration::from_secs_f32(SWOOP_DURATION),
            )));
        }
        // Son `OctopusRush` joué par le hook `on_insert` sur `OctopusMoving`.
    }
}

/// Sur `Added<OctopusFireShots>` : spawn 3 projectiles en éventail
/// (-spread / 0 / +spread degrés) vers le joueur. Le son `OctopusShoot` est
/// joué par le hook `on_insert` sur `OctopusFireShots`. Filtre
/// `Without<OctopusGreen>` car la variante verte a sa propre logique de tir.
pub fn octopus_fire_shots(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    octopus_q: Query<&Transform, (Added<OctopusFireShots>, Without<OctopusGreen>)>,
    player_tf: Single<&Transform, (With<Player>, Without<Octopus>)>,
) {
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
        // Son `OctopusShoot` joué par le hook `on_insert` sur `OctopusFireShots`.
    }
}

/// Rotation 2D d'un vecteur direction par un angle (radians).
fn rotate(dir: Vec2, angle: f32) -> Vec2 {
    let cos = angle.cos();
    let sin = angle.sin();
    Vec2::new(dir.x * cos - dir.y * sin, dir.x * sin + dir.y * cos)
}

// ═══════════════════════════════════════════════════════════════════════
//  Systèmes spécifiques à la variante verte
// ═══════════════════════════════════════════════════════════════════════

/// Variante verte du swoop : cible aléatoire sur la map (pas de relation
/// avec la position du joueur), ET intangibilité pendant le rush — collider
/// retiré + sprite assombri (mêmes effets visuels que la phase entering).
pub fn octopus_green_setup_curve(
    mut commands: Commands,
    mut octopus_q: Query<
        (Entity, &Transform, &mut Sprite),
        (With<OctopusGreen>, Added<OctopusMoving>),
    >,
    window: Single<&Window>,
) {
    let half_w = window.physical_width() as f32 / 2.0;
    let half_h = window.physical_height() as f32 / 2.0;
    let target_x_range = (half_w - TARGET_PICK_MARGIN).max(0.0);
    let target_y_range = (half_h - TARGET_PICK_MARGIN).max(0.0);

    for (entity, octopus_tf, mut sprite) in &mut octopus_q {
        let start = octopus_tf.translation.truncate();

        // Cible 100% aléatoire dans la zone jouable.
        let target = Vec2::new(
            (fastrand::f32() * 2.0 - 1.0) * target_x_range,
            (fastrand::f32() * 2.0 - 1.0) * target_y_range,
        );
        // Pass-through légèrement bruité pour donner un arc non rectiligne.
        let mid = (start + target) * 0.5;
        let jitter = Vec2::new(
            (fastrand::f32() * 2.0 - 1.0) * 100.0,
            (fastrand::f32() * 2.0 - 1.0) * 100.0,
        );
        let pass_through = mid + jitter;

        if let Ok(mut e) = commands.get_entity(entity) {
            e.try_insert(Movements::new().with(Bezier::passing_through(
                start,
                pass_through,
                target,
                Duration::from_secs_f32(SWOOP_DURATION),
            )));
            // Intangibilité pendant le rush : retire les 3 composants du
            // collider (cf. `physic/collider::collider`). Restauré au
            // retrait du marker `OctopusMoving` par `octopus_green_swoop_end`.
            e.try_remove::<Hitbox>();
            e.try_remove::<CollisionLayer>();
            e.try_remove::<CollidesWith>();
            // Bombardier actif pendant tout le swoop. Premier tir au bout
            // de `OCTOPUS_GREEN_BOMB_INTERVAL` (timer Repeating, on attend
            // que `just_finished` fire).
            e.try_insert(OctopusGreenBombThrower {
                timer: Timer::from_seconds(
                    OCTOPUS_GREEN_BOMB_INTERVAL,
                    TimerMode::Repeating,
                ),
            });
        }
        // Sprite assombri pour signaler visuellement l'immortalité.
        sprite.color = OCTOPUS_INTANGIBLE_TINT;
    }
}

/// Ré-applique la teinte sombre chaque frame tant que l'octopus vert est en
/// swoop. **Nécessaire** car d'autres systèmes (notamment `animate_hit_flash`
/// après expiration d'un `HitFlash` posé juste avant le swoop) peuvent
/// remettre `sprite.color = WHITE` pendant le rush. À enregistrer **après**
/// `animate_hit_flash` pour gagner la course "last-write-wins" du frame.
pub fn octopus_green_swoop_tint(
    mut q: Query<&mut Sprite, (With<OctopusGreen>, With<OctopusMoving>)>,
) {
    for mut sprite in &mut q {
        sprite.color = OCTOPUS_INTANGIBLE_TINT;
    }
}

/// Détecte la fin du swoop vert via `RemovedComponents<OctopusMoving>` :
/// restaure collider + alpha sprite à 1.0. Filtré sur `OctopusGreen` car le
/// standard n'a pas besoin de cette logique (jamais intangible pendant
/// swoop). Fonctionne aussi si l'octopus meurt pendant le swoop (le BT
/// retire `OctopusMoving` à la transition vers `dying`) — re-insérer un
/// collider sur un mort est inoffensif (l'entité est despawn juste après).
pub fn octopus_green_swoop_end(
    mut commands: Commands,
    mut removed: RemovedComponents<OctopusMoving>,
    mut green_q: Query<&mut Sprite, With<OctopusGreen>>,
) {
    for entity in removed.read() {
        let Ok(mut sprite) = green_q.get_mut(entity) else {
            continue;
        };
        sprite.color = Color::WHITE;
        if let Ok(mut e) = commands.get_entity(entity) {
            e.try_insert(collider(
                Shape::Circle(OCTOPUS.config.radius),
                layers::ENEMY,
                layers::PLAYER | layers::PLAYER_PROJECTILE,
            ));
            // Stoppe le bombardement (le marker est ce qui gate
            // `octopus_green_throw_bombs`).
            e.try_remove::<OctopusGreenBombThrower>();
        }
    }
}

/// Variante verte du tir : 4 projectiles **verts** en éventail plus large
/// (cf. `OCTOPUS_GREEN_SHOT_COUNT` + `OCTOPUS_GREEN_SHOT_SPREAD_DEG`). Le
/// son est joué par le hook commun `on_insert` sur `OctopusFireShots`.
pub fn octopus_green_fire_shots(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    octopus_q: Query<&Transform, (Added<OctopusFireShots>, With<OctopusGreen>)>,
    player_tf: Single<&Transform, (With<Player>, Without<Octopus>)>,
) {
    let player_pos = player_tf.translation.truncate();
    let spread = OCTOPUS_GREEN_SHOT_SPREAD_DEG.to_radians();
    // 4 angles symétriques répartis sur `[-spread, +spread]`.
    let step = (2.0 * spread) / (OCTOPUS_GREEN_SHOT_COUNT as f32 - 1.0);
    let angles: Vec<f32> = (0..OCTOPUS_GREEN_SHOT_COUNT)
        .map(|i| -spread + (i as f32) * step)
        .collect();

    for octopus_tf in &octopus_q {
        let origin = octopus_tf.translation;
        let base_dir = (player_pos - origin.truncate()).normalize_or_zero();
        if base_dir == Vec2::ZERO {
            continue;
        }
        for &angle in &angles {
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
                        color: OCTOPUS_GREEN_PROJECTILE_COLOR,
                        size: OCTOPUS_PROJECTILE_SIZE,
                    },
                    death_folder: None,
                },
            );
        }
    }
}

/// Tick chaque `OctopusGreenBombThrower` (présent uniquement pendant le swoop
/// du vert) ; à chaque `just_finished`, lance une bombe en arc parabolique
/// vers un point aléatoire de la map via une Bézier (start → apex → landing).
/// La bombe explose à la fin de son arc — cf. `octopus_green_bomb_explode`.
pub fn octopus_green_throw_bombs(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    window: Single<&Window>,
    mut thrower_q: Query<(&Transform, &mut OctopusGreenBombThrower)>,
) {
    let half_w = window.physical_width() as f32 / 2.0;
    let half_h = window.physical_height() as f32 / 2.0;
    let target_x_range = (half_w - TARGET_PICK_MARGIN).max(0.0);
    let target_y_range = (half_h - TARGET_PICK_MARGIN).max(0.0);

    for (octopus_tf, mut thrower) in &mut thrower_q {
        thrower.timer.tick(time.delta());
        if !thrower.timer.just_finished() {
            continue;
        }

        let start = octopus_tf.translation.truncate();
        // Point d'impact 100% aléatoire dans la zone jouable.
        let landing = Vec2::new(
            (fastrand::f32() * 2.0 - 1.0) * target_x_range,
            (fastrand::f32() * 2.0 - 1.0) * target_y_range,
        );
        // Apex au-dessus du milieu du segment — donne une cloche visible.
        // L'axe Y monte (convention Bevy 2D), donc on AJOUTE la hauteur.
        let apex = (start + landing) * 0.5 + Vec2::Y * OCTOPUS_GREEN_BOMB_ARC_HEIGHT;

        commands.spawn((
            Sprite {
                image: asset_server.load("images/bomb/frame000.png"),
                custom_size: Some(Vec2::splat(OCTOPUS_GREEN_BOMB_SPRITE_SIZE)),
                // Sprite bombe (skull) teinté vert pour matcher la palette
                // de l'octopus vert.
                color: OCTOPUS_GREEN_PROJECTILE_COLOR,
                ..default()
            },
            Transform::from_xyz(start.x, start.y, 0.55),
            Movements::new().with(Bezier::passing_through(
                start,
                apex,
                landing,
                Duration::from_secs_f32(OCTOPUS_GREEN_BOMB_FLIGHT_DURATION),
            )),
            OctopusGreenBomb {
                lifetime: Timer::from_seconds(
                    OCTOPUS_GREEN_BOMB_FLIGHT_DURATION,
                    TimerMode::Once,
                ),
            },
        ));
    }
}

/// Tick le `lifetime` de chaque bombe verte ; quand il termine, spawn une
/// AOE (réutilise l'anim `mine_explosion`) à la position courante de la
/// bombe = point de chute calculé par la Bézier, puis despawn la bombe.
pub fn octopus_green_bomb_explode(
    mut commands: Commands,
    time: Res<Time>,
    anim_bank: Res<crate::enemy::anim_bank::AnimBank>,
    mut query: Query<(Entity, &Transform, &mut OctopusGreenBomb)>,
) {
    for (entity, transform, mut bomb) in &mut query {
        bomb.lifetime.tick(time.delta());
        if !bomb.lifetime.is_finished() {
            continue;
        }
        spawn_aoe_animated(
            &mut commands,
            &anim_bank,
            transform.translation,
            Shape::Circle(OCTOPUS_GREEN_BOMB_AOE_RADIUS),
            OCTOPUS_GREEN_BOMB_AOE_LIFETIME,
            "octopus_green_explosion",
            OCTOPUS_GREEN_BOMB_AOE_SPRITE_SIZE,
        );
        if let Ok(mut e) = commands.get_entity(entity) {
            e.try_despawn();
        }
    }
}
