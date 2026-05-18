//! Boss — définition data-driven basée sur le framework `enemy/system.rs`.
//!
//! ## Phases
//! ```
//! entering ──timer 7s──→ active_1 ──HP<66%──→ transitioning_1 ──timer 2s──→ active_2
//!                                                                              │
//!                                                                          HP<33%
//!                                                                              ▼
//!     dead ←──on_enter despawn── dying ←──HP<1%── active_3 ←──timer 2s── transitioning_2
//! ```
//!
//! ## Parités à valider en jeu (vs ancien boss.rs)
//! - **Intro** : spirale + scaling — l'easing "progress²" matche
//! - **Musique boss** : démarre sur `on_enter active_1` (pas de délai progressif
//!   comme avant avec `boss_music_delayed`). Si tu veux le délai, ajouter une
//!   phase `idle` intermédiaire de 0.5s entre intro et active_1.
//! - **Charge** : `PatrolAndCharge` déclenche une charge tous les N secondes.
//!   L'ancien boss synchronisait au pattern (patrol 5s → charge 0.1s → patrol).
//!   La cadence est proche mais le timing peut différer de ±0.5s.
//! - **Transitions** : shake + flash OK. Spawn d'UFOs idem.
//! - **Mort** : DyingFx fait shake+flash, les **explosions aléatoires**
//!   pendant la mort ne sont PAS spawnées (limitation des behaviors &mut World).
//!   → flaggé en `TODO-VISUEL`.
//! - **Animation idle** (cycle de frames sur le sprite boss) : pas encore
//!   implémentée. Le boss reste sur `frame000.png` en Phase1/2/3.
//!   → flaggé en `TODO-VISUEL`.

use std::time::Duration;

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
use crate::game_manager::state::GameState;
use crate::geometry::shape::Shape;
use crate::menu::pause::not_paused;
use crate::movement::bounding_radius::BoundingRadius;
use crate::movement::goto::{self, Goto};
use crate::movement::movement::Movement;
use crate::movement::movement_zone::{MovementZone, amplitude_for_zone};
use crate::movement::movements::Movements;
use crate::movement::oscilate::Oscilate;
use crate::movement::rotate::RotateAround;
use crate::movement::rush::{self, Rush};
use crate::movement::shake::Shake;
use crate::movement::sinusoid::Sinusoid;
use crate::movement::spin::Spin;
use crate::movement::translate::Translate;
use crate::physic::harmless::Harmless;
use crate::physic::health::Health;
use crate::physic::invulnerable::Invulnerable;
use crate::physic::player_detection::PlayerDetection;
use crate::player::player::Player;
use crate::tweening::{Ease, Scale, Tween, TweenSequence};
use bevy::platform::collections::HashMap;

// ═══════════════════════════════════════════════════════════════════════
//  Marqueurs (utilisés par boss.rs pour le charge_movement + musique)
// ═══════════════════════════════════════════════════════════════════════

/// Marqueur présent sur l'entité boss (utilisé pour la musique + charge).
#[derive(Component)]
pub struct BossMarker;

/// Marqueur pour la musique du boss.
#[derive(Component)]
pub struct MusicBoss;

/// Vitesse de patrol latérale du boss (px/s). À transformer en variable
/// d'entité quand on voudra changer dynamiquement au runtime.
const PATROL_SPEED: f32 = 150.0;
/// Vitesse de charge du boss (px/s). Idem patrol pour la dynamicité.
const CHARGE_SPEED: f32 = 500.0;
/// Vitesse de rotation du boss pendant la charge (rad/s).
/// `4π` ≈ 2 tours par seconde.
const CHARGE_SPIN: f32 = 4.0 * std::f32::consts::PI;
/// Durée de la phase de transition entre paliers de vie (secondes). Pendant
/// cette durée, le boss est immobile et invulnérable.
const TRANSITIONING_DURATION: f32 = 2.5;

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

const INTRO_DURATION: f32 = 7.0;
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
/// Nombre de frames dans `assets/images/boss/idle/`. À mettre à jour
/// manuellement si on ajoute/retire des frames sur disque. Permet de
/// calculer la durée par frame pour qu'un cycle complet rentre pile dans
/// la phase spirale.
const BOSS_IDLE_FRAME_COUNT: usize = 11;
/// Nombre de frames dans `assets/images/boss/flexing/` (idem idle).
const BOSS_FLEXING_FRAME_COUNT: usize = 17;

const PHASE1_PATROL_SPEED_X: f32 = 200.0;
const PHASE2_PATROL_SPEED_X: f32 = 270.0;
const PHASE3_PATROL_SPEED_X: f32 = 270.0;
const PATROL_SINE_AMPLITUDE: f32 = 0.85;
const PATROL_SINE_FREQ: f32 = 4.5;
const PATROL_MARGIN: f32 = 80.0;

const CHARGE_SPEED_P1: f32 = 1500.0;
const CHARGE_SPEED_P2: f32 = 2000.0;
const CHARGE_SPEED_P3: f32 = 2500.0;

const TRANSITION_DURATION: f32 = 2.0;
const TRANSITION_SHAKE: f32 = 12.0;
const TRANSITION_UFO_COUNT_1: usize = 2;
const TRANSITION_UFO_COUNT_2: usize = 4;

const DYING_DURATION: f32 = 4.0;
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
        difficulty: &ResMut<Difficulty>,
        spawn_pos: SpawnPosition,
        asset_server: &Res<AssetServer>,
    ) {
        println!("going to spawn boss");
        let spiral = Movements::new()
            .with(RotateAround::new(Vec2::ZERO, INTRO_SPIRAL_TURNS))
            .with(Goto::new(Vec2::ZERO, INTRO_GOTO_SPEED));
        // Frame durations alignées sur les durées de phase : un cycle complet
        // d'animation joue pile pendant chaque phase. Anciennement "idle" avec
        // 3s/frame (ne tournait jamais en 3s de phase + nom incorrect) et
        // flexing à 0.1s/frame (cycle de 1.7s coupé à 1s).
        let idle_frame_duration =
            Duration::from_secs_f32(INTRO_SPIRAL_DURATION / BOSS_IDLE_FRAME_COUNT as f32);
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
        //.with(BehaviorBuilder::from_component(AudioBundle {
        //    source: asset_server.load("audio/sfx/boss_start.ogg"),
        //    settings: PlaybackSettings::DESPAWN,
        //}));
        let boss_anim_behavior =
            BehaviorBuilder::from_component(Animation::new("boss", Duration::from_secs_f32(0.1)));
        let boss_idle_anim_behavior = BehaviorBuilder::from_component(Animation::new(
            "boss_idle",
            Duration::from_secs_f32(0.1),
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

        // Phase de transition entre paliers de vie : boss immobile + invulnérable
        // pendant TRANSITIONING_DURATION, puis le timer du OrderedNodeList expire,
        // `on_complete` pousse "transition_done", la choice retourne en patrol.
        let transitioning = BehaviorBuilder::first(
            Duration::from_secs_f32(TRANSITIONING_DURATION),
            BehaviorBuilder::multiple()
                .with(BehaviorBuilder::from_component(Movements::new()))
                .with(BehaviorBuilder::from_component(Invulnerable)),
        )
        .on_complete("transition_done");

        // Ordre = priorité quand plusieurs transitions matchent dans la même
        // frame. player_charge déclaré AVANT wall_* pour qu'une charge gagne
        // sur un rebond mur (cas typique : boss en patrol_right plaqué au mur
        // droit, joueur entre dans la zone → on veut la charge, pas le rebond).
        // Les transitions wall_* depuis index 2 (charge) renvoient vers le
        // patrol qui s'éloigne du mur touché → bounce naturel.
        // Ordre = priorité quand plusieurs transitions matchent dans la même
        // frame. hp_threshold déclaré EN TÊTE pour gagner sur tout le reste
        // (le boss doit toujours basculer en transitioning quand un seuil de
        // vie est franchi, même au milieu d'un wall hit ou d'un player_charge).
        let alive = BehaviorBuilder::choice()
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
        let dying = BehaviorBuilder::first(
            Duration::from_secs_f32(0.4),
            BehaviorBuilder::from_component(Movements::new().with(Shake::new(100.0, 0.4))),
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
        ));
    }

    fn name(&self) -> &str {
        "boss"
    }
}

/// Surveille les PV du boss et pousse `"hp_threshold"` une fois lors du
/// franchissement de chaque palier (2/3 puis 1/3). Skippé pendant la
/// transitioning (présence du marker `Invulnerable`) → pas de re-push tant
/// que la phase précédente n'est pas finie, donc même si le boss saute deux
/// paliers en un seul gros hit, les deux transitionings se déclencheront en
/// séquence.
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
