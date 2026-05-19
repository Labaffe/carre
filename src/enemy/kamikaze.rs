//! Kamikaze — ennemi suicide qui poursuit le joueur puis explose.
//!
//! Cycle :
//! 1. **pursuing** : poursuit le joueur en continu (`Chase`). Joue l'animation
//!    `kamikaze_chase` en one-shot : frames 000→001→002→003, puis reste figé
//!    sur 003 (flammes "armées"). `PlayerDetection` cercle close-range guette.
//! 2. **armed** : déclenché quand le joueur entre dans la zone. Le kamikaze
//!    **continue à poursuivre** le joueur, clignote rouge, et joue
//!    l'animation `kamikaze_explode` (frames 004→014) sur la durée du
//!    countdown. L'animation et le countdown sont alignés : début et fin
//!    simultanés. À `on_complete`, transition vers `booming`.
//! 3. **booming** : insère `KamikazeBoom`. `kamikaze_boom_system` spawn l'AOE,
//!    joue le son d'explosion, et despawn le kamikaze.
//!
//! Le kamikaze est **vulnérable** (Health=2) pendant pursuing/armed : si le
//! joueur le tue avant le boom, il meurt silencieusement (outer choice →
//! dying, sans AOE).

use std::time::Duration;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::audio::{Sfx, SfxPlayer};
use crate::behavior::BehaviorBuilder;
use crate::behavior::behavior::BehaviorComponent;
use crate::behavior::choice_list::TransitionMessages;
use crate::enemy::anim_bank::Animation;
use crate::enemy::death::DespawnSelf;
use crate::enemy::enemies::KAMIKAZE;
use crate::enemy::enemy::Enemy;
use crate::enemy::enemy_builder::EnemyBuilder;
use crate::enemy::mine::BlinkRed;
use crate::game_manager::difficulty::{Difficulty, SpawnPosition};
use crate::geometry::shape::Shape;
use crate::item::item::{DropTable, ItemType};
use crate::movement::chase::Chase;
use crate::movement::despawn_off_screen::DespawnOffScreen;
use crate::movement::movements::Movements;
use crate::physic::area_of_effect::{spawn_aoe, AoeAssets};
use crate::physic::health::Health;
use crate::physic::player_detection::PlayerDetection;
use crate::sprite_orient::FaceMovement;

/// Vitesse de poursuite de base (px/s). Esquivable au début, devient
/// progressivement plus rapide via `KamikazeSpeedRamp`.
const KAMIKAZE_CHASE_SPEED: f32 = 350.0;
/// Bonus de vitesse gagné par seconde de vie (px/s par seconde). Cumulé
/// linéairement avec `KAMIKAZE_CHASE_SPEED`. Plus le kamikaze survit
/// longtemps, plus il devient dangereux — pousse à le prioriser.
const KAMIKAZE_RAMP_RATE: f32 = 120.0;
/// Bonus de vitesse maximum atteint (px/s). Plafond pour éviter la course
/// folle. Atteint après `MAX_BONUS / RAMP_RATE` secondes de vie.
const KAMIKAZE_MAX_BONUS: f32 = 1000.0;
/// Rayon de détection joueur (px) qui déclenche le countdown. Close range.
const KAMIKAZE_DETECTION_RADIUS: f32 = 100.0;
/// Rayon de l'AOE explosion (px). Légèrement plus large que la détection.
const KAMIKAZE_AOE_RADIUS: f32 = 180.0;
/// Durée de vie de l'AOE (secondes). Bref — pas zone denial, juste un
/// gros punch instantané.
const KAMIKAZE_AOE_LIFETIME: f32 = 0.8;
/// Période du clignotement rouge pendant le countdown.
const KAMIKAZE_BLINK_PERIOD: f32 = 0.15;
/// Durée par frame des animations (chase warm-up + explosion).
const KAMIKAZE_FRAME_DURATION: f32 = 0.08;
/// Nombre de frames de l'animation d'explosion (dossier `explode/`).
const KAMIKAZE_EXPLODE_FRAME_COUNT: f32 = 11.0;
/// Durée du countdown avant explosion (secondes). Alignée sur la durée de
/// l'animation d'explosion : l'animation commence au début du countdown et
/// se termine à la frame finale exactement quand le boom se déclenche.
const KAMIKAZE_COUNTDOWN_DURATION: f32 =
    KAMIKAZE_FRAME_DURATION * KAMIKAZE_EXPLODE_FRAME_COUNT;

static KAMIKAZE_DROP_TABLE: [(ItemType, f32); 2] =
    [(ItemType::Bomb, 0.10), (ItemType::BonusScore, 0.15)];

/// Marqueur sur l'entité Kamikaze.
#[derive(Component)]
pub struct Kamikaze;

/// Inséré à l'entrée de l'état exploding. `kamikaze_boom_system` détecte
/// l'`Added` et déclenche AOE + son + reset couleur — une seule fois.
#[derive(Component, Clone)]
pub struct KamikazeBoom;

/// Inséré à l'entrée de l'état armed. `kamikaze_scream_system` détecte
/// l'`Added` et joue le cri terrifiant — une seule fois par kamikaze, au
/// moment où il devient dangereux pour le joueur.
#[derive(Component, Clone)]
pub struct KamikazeArmed;

/// Inséré pendant la phase pursuing. `kamikaze_laugh_start_system` spawn
/// un AudioPlayer en loop (le rire) attaché à l'entité. Quand le composant
/// est retiré (transition vers armed), `kamikaze_laugh_stop_system` despawn
/// l'audio. Cleanup automatique aussi si le kamikaze est tué.
#[derive(Component, Clone)]
pub struct KamikazeLaughing;

/// Marker sur l'entité audio du rire — permet de retrouver et despawn cet
/// audio précis quand la phase pursuing se termine.
#[derive(Component)]
pub struct KamikazeLaughAudio;

/// Tick l'âge du kamikaze et applique un déplacement additionnel vers le
/// joueur dont la magnitude grandit avec l'âge. S'additionne au `Chase` de
/// base — un kamikaze "ancien" est sensiblement plus rapide qu'un fraîchement
/// spawnée, créant la pression "prioriser ou souffrir".
#[derive(Component)]
pub struct KamikazeSpeedRamp {
    elapsed: f32,
}
impl KamikazeSpeedRamp {
    pub fn new() -> Self {
        Self { elapsed: 0.0 }
    }
}

pub struct KamikazeBuilder {
    timer: Timer,
}
impl KamikazeBuilder {
    pub fn new() -> Self {
        Self {
            timer: Timer::new(Duration::ZERO, TimerMode::Once),
        }
    }
}
impl EnemyBuilder for KamikazeBuilder {
    fn get_timer(&mut self) -> &mut Timer {
        &mut self.timer
    }
    fn name(&self) -> &str {
        "kamikaze"
    }
    fn preload_anim(&self) -> HashMap<&str, &str> {
        HashMap::from([
            ("kamikaze_chase", "images/kamikaze/chase"),
            ("kamikaze_explode", "images/kamikaze/explode"),
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
        let pos = spawn_pos.resolve(window, KAMIKAZE.config.sprite_size / 2.0);
        let frame_dur = Duration::from_secs_f32(KAMIKAZE_FRAME_DURATION);

        // Phase 0 : chase + animation 000→003 one-shot (reste figé sur 003).
        // `KamikazeLaughing` marker → spawn d'un AudioPlayer LOOP via le
        // système `kamikaze_laugh_start_system`. Retiré à la transition.
        let pursuing = BehaviorBuilder::multiple()
            .with(BehaviorBuilder::from_component(
                Movements::new().with(Chase::new(KAMIKAZE_CHASE_SPEED)),
            ))
            .with(BehaviorBuilder::from_component(
                Animation::new("kamikaze_chase", frame_dur).one_shot(),
            ))
            .with(BehaviorBuilder::from_component(KamikazeLaughing));

        // Phase 1 : countdown. Chase continue, BlinkRed actif, animation
        // d'explosion joue 004→014 sur toute la durée. Le marker `KamikazeArmed`
        // déclenche le son d'alerte (one-shot via `Added`). Quand l'animation
        // termine (= countdown terminé), `on_complete("boom")` → phase 2.
        let armed = BehaviorBuilder::first(
            Duration::from_secs_f32(KAMIKAZE_COUNTDOWN_DURATION),
            BehaviorBuilder::multiple()
                .with(BehaviorBuilder::from_component(
                    Movements::new().with(Chase::new(KAMIKAZE_CHASE_SPEED)),
                ))
                .with(BehaviorBuilder::from_component(BlinkRed::new(
                    KAMIKAZE_BLINK_PERIOD,
                )))
                .with(BehaviorBuilder::from_component(KamikazeArmed))
                .with(BehaviorBuilder::from_component(
                    Animation::new("kamikaze_explode", frame_dur).one_shot(),
                )),
        )
        .on_complete("boom");

        // Phase 2 : `KamikazeBoom` déclenche AOE + son + despawn (one-shot
        // via `Added<>`).
        let booming = BehaviorBuilder::from_component(KamikazeBoom);

        let alive_choice = BehaviorBuilder::choice()
            .with(pursuing) // 0
            .with(armed) // 1
            .with(booming) // 2
            .add_transition(0, 1, "player_detected")
            .add_transition(1, 2, "boom");

        // Mort : DespawnSelf, soit après l'explosion (on_complete "die"), soit
        // si tué par tirs avant (HP=0 → "die" via detect_death).
        let dying = BehaviorBuilder::from_component(DespawnSelf);

        let behavior = BehaviorBuilder::choice()
            .with(alive_choice)
            .with(dying)
            .add_transition(0, 1, "die");

        commands.spawn((
            Sprite {
                image: asset_server.load("images/kamikaze/chase/frame000.png"),
                custom_size: Some(Vec2::splat(KAMIKAZE.config.sprite_size)),
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y, 0.5),
            TransitionMessages::new(),
            Enemy::new(KAMIKAZE),
            Health::new(KAMIKAZE.total_hp),
            DespawnOffScreen,
            Kamikaze,
            KamikazeSpeedRamp::new(),
            FaceMovement::faces_left(),
            PlayerDetection {
                shape: Shape::Circle(KAMIKAZE_DETECTION_RADIUS),
                on_enter: Some("player_detected"),
                on_exit: None,
                inside: false,
                cooldown_duration: 0.0,
                cooldown_remaining: 0.0,
            },
            BehaviorComponent::new(behavior),
            DropTable {
                drops: &KAMIKAZE_DROP_TABLE,
            },
        ));
    }
}

/// Applique un déplacement additionnel vers le joueur dont la magnitude
/// grandit avec l'âge du kamikaze (`bonus = age * RAMP_RATE`, plafonné à
/// `MAX_BONUS`). S'additionne au `Chase` standard.
pub fn kamikaze_speed_ramp_system(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut KamikazeSpeedRamp)>,
    player_q: Query<&Transform, (With<crate::player::player::Player>, Without<KamikazeSpeedRamp>)>,
) {
    let Ok(player_tf) = player_q.single() else { return };
    let dt = time.delta_secs();
    let player_pos = player_tf.translation.xy();

    for (mut tf, mut ramp) in &mut query {
        ramp.elapsed += dt;
        let bonus = (ramp.elapsed * KAMIKAZE_RAMP_RATE).min(KAMIKAZE_MAX_BONUS);
        let dir = (player_pos - tf.translation.xy()).normalize_or_zero();
        let delta = dir * bonus * dt;
        tf.translation.x += delta.x;
        tf.translation.y += delta.y;
    }
}

/// Détecte `Added<KamikazeBoom>` (insertion à l'entrée de la phase booming,
/// déclenchée par `on_complete` du countdown) : spawn l'AOE, joue le son,
/// despawn le kamikaze. Une seule fois par entité grâce à `Added`.
pub fn kamikaze_boom_system(
    mut commands: Commands,
    mut sfx: SfxPlayer,
    aoe_assets: Res<AoeAssets>,
    query: Query<(Entity, &Transform), Added<KamikazeBoom>>,
) {
    for (entity, transform) in &query {
        spawn_aoe(
            &mut commands,
            &aoe_assets,
            transform.translation,
            Shape::Circle(KAMIKAZE_AOE_RADIUS),
            KAMIKAZE_AOE_LIFETIME,
        );
        sfx.play(Sfx::Explosion);
        commands.entity(entity).try_despawn();
    }
}

/// Détecte `Added<KamikazeArmed>` : joue le cri terrifiant une fois quand
/// le kamikaze entre en phase armed.
pub fn kamikaze_scream_system(
    mut sfx: SfxPlayer,
    query: Query<(), Added<KamikazeArmed>>,
) {
    for _ in &query {
        sfx.play(Sfx::KamikazeScream);
    }
}

/// Détecte `Added<KamikazeLaughing>` : spawn un AudioPlayer LOOP en CHILD du
/// kamikaze. La relation parent-enfant assure que si le kamikaze est tué,
/// l'audio est despawn en cascade (try_despawn récursif).
pub fn kamikaze_laugh_start_system(
    mut commands: Commands,
    library: Res<crate::audio::SfxLibrary>,
    query: Query<Entity, Added<KamikazeLaughing>>,
) {
    for kamikaze_entity in &query {
        if let Ok(mut e) = commands.get_entity(kamikaze_entity) {
            e.with_children(|parent| {
                parent.spawn((
                    AudioPlayer::new(library.get(Sfx::KamikazeLaugh)),
                    PlaybackSettings::LOOP,
                    KamikazeLaughAudio,
                ));
            });
        }
    }
}

/// Détecte la disparition de `KamikazeLaughing` (transition pursuing → armed).
/// Cherche l'audio enfant via le marker `KamikazeLaughAudio` et despawn.
/// Si le kamikaze est mort, la cascade parent-enfant a déjà géré le despawn —
/// `children_q.get` échoue silencieusement.
pub fn kamikaze_laugh_stop_system(
    mut commands: Commands,
    mut removed: RemovedComponents<KamikazeLaughing>,
    children_q: Query<&Children>,
    audio_q: Query<(), With<KamikazeLaughAudio>>,
) {
    for kamikaze_entity in removed.read() {
        let Ok(children) = children_q.get(kamikaze_entity) else { continue };
        for &child in children {
            if audio_q.contains(child) {
                if let Ok(mut e) = commands.get_entity(child) {
                    e.try_despawn();
                }
            }
        }
    }
}
