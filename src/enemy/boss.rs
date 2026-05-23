//! Boss — définition data-driven montée sur le framework de behaviors.
//!
//! ## Flow général
//! ```
//! entering (spirale + flexing, 5.5s)
//!     │ wrapper multiple : Invulnerable + Harmless + TweenSequence<Scale>
//!     │ scale 0.01 → 1.0 sur la spirale (3s), boss arrive à (0,0)
//!     ▼
//! alive (multiple) ── die ──→ dying (Shake 4s) ──→ DespawnSelf
//!     ├─ alive_choice :
//!     │     0: patrol_left (Oscilate + Translate gauche)
//!     │     1: patrol_right (idem droite)
//!     │     2: charge (Rush::on_axis(X) + Spin auto_reset, ends on wall_left/right)
//!     │     3: transitioning (immobile + Shake + Invulnerable, 2.5s, on_complete)
//!     └─ Animation "boss_idle" en parallèle (continue à travers les transitions)
//! ```
//!
//! ## Paliers de vie
//! `boss_hp_threshold_check` surveille `Health.fraction()` et pousse
//! `"hp_threshold"` quand le boss franchit 2/3 puis 1/3 (skip pendant
//! transitioning grâce au check `With<Invulnerable>` → pas de re-push tant que
//! la phase précédente n'est pas finie). `BossPhaseTracker` séquence les
//! franchissements.

use std::time::Duration;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::behavior::BehaviorBuilder;
use crate::behavior::behavior::BehaviorComponent;
use crate::behavior::choice_list::TransitionMessages;
use crate::enemy::anim_bank::Animation;
use crate::enemy::death::DespawnSelf;
use crate::enemy::enemies::BOSS;
use crate::enemy::enemy::Enemy;
use crate::enemy::enemy_builder::EnemyBuilder;
use crate::game_manager::difficulty::{Difficulty, SpawnPosition};
use crate::geometry::shape::Shape;
use crate::movement::bounding_radius::BoundingRadius;
use crate::movement::goto::Goto;
use crate::movement::movement_zone::{MovementZone, amplitude_for_zone};
use crate::movement::movements::Movements;
use crate::movement::oscilate::Oscilate;
use crate::movement::rotate::RotateAround;
use crate::movement::rush::Rush;
use crate::movement::shake::Shake;
use crate::movement::spin::Spin;
use crate::movement::translate::Translate;
use crate::physic::collider::{collider, layers};
use crate::physic::harmless::Harmless;
use crate::physic::health::Health;
use crate::physic::invulnerable::Invulnerable;
use crate::physic::player_detection::PlayerDetection;
use crate::tweening::{Ease, Scale, Tween, TweenSequence};

// ═══════════════════════════════════════════════════════════════════════
//  Marqueurs
// ═══════════════════════════════════════════════════════════════════════

/// Marqueur présent sur l'entité boss. Sert au filtrage de queries (musique,
/// HP threshold check, debug overlay, etc.).
#[derive(Component)]
pub struct BossMarker;

/// Marqueur sur l'entité audio qui joue la musique du boss.
#[derive(Component)]
#[require(crate::GameplayEntity)]
pub struct MusicBoss;

/// Suivi du palier de vie courant du boss. Démarre à 1.
/// - `phase = 1` : tant que les PV sont au-dessus de 2/3.
/// - `phase = 2` : entre 2/3 et 1/3 (après la 1re transition).
/// - `phase = 3` : sous 1/3 (après la 2e transition).
/// Incrémenté uniquement par `boss_hp_threshold_check` au franchissement,
/// pas par les dégâts directs.
#[derive(Component)]
pub struct BossPhaseTracker {
    pub phase: u8,
}

impl BossPhaseTracker {
    pub fn new() -> Self {
        Self { phase: 1 }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Constantes
// ═══════════════════════════════════════════════════════════════════════

// ─── Intro (spirale + flexing) ──────────────────────────────────────────
/// Durée de la phase de spirale (secondes). Le scale tween dure pareil pour
/// que le boss atteigne sa taille finale à la fin de la spirale.
const INTRO_SPIRAL_DURATION: f32 = 3.0;
/// Durée de la phase de flexing après la spirale (secondes).
const INTRO_FLEXING_DURATION: f32 = 2.5;
/// Y de spawn du boss avant l'intro. Le boss tombe en spirale depuis ce point
/// vers (0, 0). Définit aussi le rayon initial de la spirale.
const INTRO_SPAWN_Y: f32 = 400.0;
const INTRO_START_SCALE: f32 = 0.01;
const INTRO_END_SCALE: f32 = 1.0;
/// Vitesse de rotation pendant la spirale d'intro (tours/seconde). À 0.5 et
/// avec une durée de 3s on obtient 1.5 tours visuels — lisible sans tourner
/// la tête au joueur.
const INTRO_SPIRAL_TURNS: f32 = 0.5;
/// Vitesse d'attraction du `Goto` (px/s). Dérivée de `INTRO_SPAWN_Y` et
/// `INTRO_SPIRAL_DURATION` pour que le boss arrive au centre PILE à la fin
/// de la phase — pas de temps mort statique au centre.
const INTRO_GOTO_SPEED: f32 = INTRO_SPAWN_Y / INTRO_SPIRAL_DURATION;

// ─── Animations (durées par frame) ──────────────────────────────────────
/// Durée par frame de l'animation idle (secondes). Utilisée à la fois pendant
/// la spirale d'intro et pendant tout `alive` (patrol/charge/transitioning).
/// Avec 11 frames sur disque, cycle complet en 11 * 0.1 = 1.1s.
const BOSS_IDLE_FRAME_DURATION: f32 = 0.1;
/// Nombre de frames dans `assets/images/boss/flexing/`. À mettre à jour
/// manuellement si on ajoute/retire des frames sur disque. Permet de
/// calculer la durée par frame pour qu'un cycle complet rentre pile dans
/// la phase flexing.
const BOSS_FLEXING_FRAME_COUNT: usize = 17;

// ─── Combat (alive) ─────────────────────────────────────────────────────
/// Vitesse de patrol latérale du boss (px/s).
const PATROL_SPEED: f32 = 150.0;
/// Vitesse de charge du boss (px/s).
const CHARGE_SPEED: f32 = 500.0;
/// Vitesse de rotation du boss pendant la charge (rad/s). `4π` ≈ 2 tours/s.
const CHARGE_SPIN: f32 = 4.0 * std::f32::consts::PI;
/// Durée d'une transition entre paliers de vie (secondes). Le boss est
/// immobile, invulnérable, et tremble pendant cette durée.
const TRANSITIONING_DURATION: f32 = 2.5;
/// Amplitude max du shake pendant la transitioning (px, atteinte à la fin
/// — croissance quadratique depuis 0).
const TRANSITION_SHAKE: f32 = 12.0;

// ─── Mort ───────────────────────────────────────────────────────────────
/// Durée du shake de mort avant DespawnSelf (secondes).
const DYING_DURATION: f32 = 2.0;
/// Amplitude max du shake de mort (px, atteinte en fin de phase).
const DYING_SHAKE_MAX: f32 = 20.0;

// ═══════════════════════════════════════════════════════════════════════
//  Définition du boss
// ═══════════════════════════════════════════════════════════════════════

pub struct BossBuilder {
    timer: Timer,
}
impl BossBuilder {
    pub fn new() -> Self {
        Self {
            timer: Timer::new(Duration::ZERO, TimerMode::Once),
        }
    }
}
impl EnemyBuilder for BossBuilder {
    fn get_timer(&mut self) -> &mut Timer {
        &mut self.timer
    }

    fn preload_anim(&self) -> HashMap<&str, &str> {
        HashMap::from([
            ("boss", "images/boss/animation_1"),
            ("boss_flexing", "images/boss/flexing"),
            ("boss_idle", "images/boss/idle"),
        ])
    }

    fn spawn(
        &self,
        mut commands: Commands,
        window: &Window,
        _difficulty: &ResMut<Difficulty>,
        _spawn_pos: SpawnPosition,
        asset_server: &Res<AssetServer>,
    ) {
        let spiral = Movements::new()
            .with(RotateAround::new(Vec2::ZERO, INTRO_SPIRAL_TURNS))
            .with(Goto::new(Vec2::ZERO, INTRO_GOTO_SPEED));
        // Idle : durée fixe (utilisée dans spirale ET alive). Cycle ~1.1s à
        // 0.1s/frame. Flexing : dérivé pour que le cycle entier rentre pile
        // dans la phase flexing (animation jouée une fois, pas coupée).
        let idle_frame_duration = Duration::from_secs_f32(BOSS_IDLE_FRAME_DURATION);
        let flexing_frame_duration =
            Duration::from_secs_f32(INTRO_FLEXING_DURATION / BOSS_FLEXING_FRAME_COUNT as f32);
        let entering_sequence = BehaviorBuilder::first(
            Duration::from_secs_f32(INTRO_SPIRAL_DURATION),
            BehaviorBuilder::multiple()
                .with(BehaviorBuilder::from_component(spiral))
                .with(BehaviorBuilder::from_component(Animation::new(
                    "boss_idle",
                    idle_frame_duration,
                ))),
        )
        .then(
            Duration::from_secs_f32(INTRO_FLEXING_DURATION),
            BehaviorBuilder::from_component(Animation::new("boss_flexing", flexing_frame_duration)),
        );
        // Pendant toute la durée d'entering (spirale + flexing), le boss est
        // invulnérable ET inoffensif au contact. Les deux markers sont insérés
        // en parallèle de la séquence et retirés automatiquement quand le
        // wrapping `multiple()` se disable (fin de l'entering top-level).
        // Le TweenSequence<Scale> fait grossir le boss de INTRO_START_SCALE
        // à INTRO_END_SCALE sur la durée de la spirale (3s). Une fois fini,
        // le tween_system retire automatiquement le composant — le boss reste
        // à INTRO_END_SCALE pour le flexing puis l'active.
        let entering = BehaviorBuilder::multiple()
            .with(entering_sequence)
            .with(BehaviorBuilder::from_component(Invulnerable))
            .with(BehaviorBuilder::from_component(Harmless))
            .with(BehaviorBuilder::from_component(
                TweenSequence::<Scale>::new(Tween::new(
                    INTRO_START_SCALE,
                    INTRO_END_SCALE,
                    INTRO_SPIRAL_DURATION,
                    Ease::InQuad,
                )),
            ));

        // Amplitude verticale : pile la hauteur de la zone effective (margin y + radius),
        // pour que l'oscillation ne pousse jamais contre le clamp du movement_driver.
        // Doit rester cohérent avec le MovementZone et BoundingRadius en bas de cette fn.
        let bounding_r = BOSS.config.sprite_size / 2.0;
        let amplitude_y = amplitude_for_zone(0.0, bounding_r, window.physical_height() as f32);
        let patrol_movement_left = Movements::new()
            .with(Oscilate::new(
                Vec2::new(1.0, 0.0),
                Vec2::ZERO,
                20.0,
                amplitude_y,
            ))
            .with(Translate::new(Vec2::new(-1.0, 0.0), PATROL_SPEED));
        let patrol_movement_right = Movements::new()
            .with(Oscilate::new(
                Vec2::new(1.0, 0.0),
                Vec2::ZERO,
                20.0,
                amplitude_y,
            ))
            .with(Translate::new(Vec2::new(1.0, 0.0), PATROL_SPEED));

        // Charge déclenchée par PlayerDetection : Rush::on_axis(X) fige la
        // direction horizontale (gauche/droite) à la 1re frame selon le côté
        // du joueur. Spin::with_auto_reset fait tourner le boss sur lui-même
        // pendant la charge et remet la rotation à 0 quand le composant est
        // retiré (via hook on_remove). La charge se termine quand le boss
        // touche un bord (rising edge wall_left/wall_right via MovementZone).
        let charge_movement = Movements::new().with(Rush::new(CHARGE_SPEED).on_axis(Vec2::X));
        let charge = BehaviorBuilder::multiple()
            .with(BehaviorBuilder::from_component(charge_movement))
            .with(BehaviorBuilder::from_component(
                Spin::new(CHARGE_SPIN).with_auto_reset(),
            ));

        // Phase de transition entre paliers de vie : boss invulnérable +
        // tremble pendant TRANSITIONING_DURATION. La Shake remplace l'ancien
        // Movements vide ; le Shake garde le boss centré (offset random ±A
        // chaque frame, pas de drift). À la fin, `on_complete` pousse
        // "transition_done", la choice retourne en patrol.
        let transitioning = BehaviorBuilder::first(
            Duration::from_secs_f32(TRANSITIONING_DURATION),
            BehaviorBuilder::multiple()
                .with(BehaviorBuilder::from_component(
                    Movements::new().with(Shake::new(TRANSITION_SHAKE, TRANSITIONING_DURATION)),
                ))
                .with(BehaviorBuilder::from_component(Invulnerable)),
        )
        .on_complete("transition_done");

        // Ordre des transitions = priorité quand plusieurs matchent dans la
        // même frame :
        // - `hp_threshold` EN TÊTE : le palier de vie gagne toujours, même
        //   au milieu d'un wall hit ou d'un player_charge.
        // - `player_charge` avant les `wall_*` : un boss plaqué au mur qui
        //   détecte le joueur doit charger, pas rebondir.
        // - `wall_*` depuis charge (index 2) renvoie au patrol qui s'éloigne
        //   du mur touché → bounce naturel.
        let alive_choice = BehaviorBuilder::choice()
            .with(BehaviorBuilder::from_component(patrol_movement_left)) // 0
            .with(BehaviorBuilder::from_component(patrol_movement_right)) // 1
            .with(charge) // 2
            .with(transitioning) // 3
            .add_transition(0, 3, "hp_threshold")
            .add_transition(1, 3, "hp_threshold")
            .add_transition(2, 3, "hp_threshold")
            .add_transition(3, 0, "transition_done")
            .add_transition(0, 2, "player_charge")
            .add_transition(1, 2, "player_charge")
            .add_transition(2, 1, "wall_left")
            .add_transition(2, 0, "wall_right")
            .add_transition(0, 1, "wall_left")
            .add_transition(1, 0, "wall_right");
        // Wrap parallèle : l'animation idle tourne en boucle pendant tout
        // `alive` (patrol/charge/transitioning). Le choice ne touche que les
        // composants de mouvement, pas la Sprite — donc l'Animation reste
        // continue à travers les transitions internes du choice.
        let alive =
            BehaviorBuilder::multiple()
                .with(alive_choice)
                .with(BehaviorBuilder::from_component(Animation::new(
                    "boss_idle",
                    idle_frame_duration,
                )));
        // Phase de mort : shake pendant DYING_DURATION (amplitude qui croît
        // quadratiquement de 0 à DYING_SHAKE_MAX), puis DespawnSelf.
        let dying = BehaviorBuilder::first(
            Duration::from_secs_f32(DYING_DURATION),
            BehaviorBuilder::from_component(
                Movements::new().with(Shake::new(DYING_SHAKE_MAX, DYING_DURATION)),
            ),
        )
        .then(
            Duration::from_secs_f32(1.0),
            BehaviorBuilder::from_component(DespawnSelf),
        );
        let life = BehaviorBuilder::choice()
            .with(alive)
            .with(dying)
            .add_transition(0, 1, "die");
        // Durée totale de l'entering = somme des phases internes. DOIT matcher,
        // sinon le ParallelNodeList wrapper coupe les composants avant la fin
        // (TweenSequence<Scale>, Invulnerable, Harmless) et le boss reste à un
        // scale intermédiaire pour tout le combat.
        let behavior = BehaviorBuilder::first(
            Duration::from_secs_f32(INTRO_SPIRAL_DURATION + INTRO_FLEXING_DURATION),
            entering,
        )
        .then(Duration::from_secs_f32(10000.0), life);
        commands.spawn((
            Sprite {
                image: asset_server.load("images/boss/idle/frame000.png"),
                custom_size: Some(Vec2::splat(BOSS.config.sprite_size)),
                color: Color::WHITE,
                ..default()
            },
            Transform {
                translation: Vec3::new(0.0, INTRO_SPAWN_Y, 0.0),
                scale: Vec3::splat(INTRO_START_SCALE),
                ..default()
            },
            Enemy::new(BOSS),
            Health::new(BOSS.total_hp),
            BossMarker,
            BossPhaseTracker::new(),
            TransitionMessages::new(),
            BoundingRadius(BOSS.config.sprite_size / 2.0),
            MovementZone::new(Vec2::new(0.0, 0.0))
                .with_left("wall_left")
                .with_right("wall_right"),
            PlayerDetection {
                shape: Shape::Rect {
                    half_width: window.physical_width() as f32 / 2.0 + 200.0,
                    half_length: 40.0,
                },
                on_enter: Some("player_charge"),
                on_exit: None,
                inside: false,
                cooldown_duration: 3.0,
                cooldown_remaining: 0.0,
            },
            BehaviorComponent::new(behavior),
            collider(
                Shape::Circle(BOSS.config.radius),
                layers::ENEMY,
                layers::PLAYER | layers::PLAYER_PROJECTILE,
            ),
        ));
    }

    fn name(&self) -> &'static str {
        "boss"
    }
}

/// Surveille les PV du boss et pousse `"hp_threshold"` une fois lors du
/// franchissement de chaque palier (2/3 puis 1/3). Skippé pendant la
/// transitioning (présence du marker `Invulnerable`) → pas de re-push tant
/// que la phase précédente n'est pas finie, donc même si le boss saute deux
/// paliers en un seul gros hit, les deux transitionings se déclencheront en
/// séquence.
/// Observer : à chaque `EnemyDeathEvent` trigger, si l'entité morte porte
/// `BossMarker`, déclenche un screen shake épique (synchrone avec la phase
/// dying du behavior tree qui shake le sprite pendant `DYING_DURATION`).
pub fn boss_death_screen_shake(
    trigger: On<crate::enemy::enemy::EnemyDeathEvent>,
    mut commands: Commands,
    boss_q: Query<(), With<BossMarker>>,
) {
    let ev = trigger.event();
    if boss_q.get(ev.entity).is_err() { return; }
    commands.trigger(crate::fx::screen_shake::ScreenShakeEvent::BOSS_DEATH);
    commands.trigger(crate::fx::time_fx::TimeFxEvent::SLOWMO_BOSS_KILL);
}

pub fn boss_hp_threshold_check(
    invul_q: Query<(), With<Invulnerable>>,
    mut q: Query<
        (
            Entity,
            &Health,
            &mut TransitionMessages,
            &mut BossPhaseTracker,
        ),
        With<BossMarker>,
    >,
) {
    for (entity, health, mut messages, mut tracker) in &mut q {
        if invul_q.contains(entity) {
            continue;
        }
        let f = health.fraction();
        if tracker.phase == 1 && f <= 2.0 / 3.0 {
            messages.messages.push("hp_threshold".to_string());
            tracker.phase = 2;
        } else if tracker.phase == 2 && f <= 1.0 / 3.0 {
            messages.messages.push("hp_threshold".to_string());
            tracker.phase = 3;
        }
    }
}
