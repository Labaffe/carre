//! Système de niveau — Timeline déclarative d'événements.
//!
//! Un niveau est une séquence d'étapes (`LevelStep`). Chaque étape a :
//! - Un **déclencheur** : temps absolu ou délai après l'étape précédente
//! - Une liste d'**actions** : ce qui se passe quand l'étape se déclenche
//!
//! Le `LevelRunner` parcourt les étapes dans l'ordre et exécute les actions.
//! Cela remplace les dizaines de booléens et timers éparpillés dans difficulty.rs.
//!
//! ## Exemple
//! ```ignore
//! LevelStep::at(7.0, "countdown")
//!     .with(Action::PlaySound("audio/charging.ogg"))
//!     .with(Action::StartCountdown)
//! ```

use std::collections::HashMap;

use crate::difficulty::BoomEvent;
use crate::pause::not_paused;
use crate::state::GameState;
use bevy::prelude::*;

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), setup_level)
            .add_systems(
                Update,
                run_level
                    .run_if(in_state(GameState::Playing))
                    .run_if(not_paused),
            );
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Types
// ═══════════════════════════════════════════════════════════════════════

/// Quand une étape doit se déclencher.
#[derive(Clone)]
pub enum Trigger {
    /// Se déclenche à un temps absolu de jeu (secondes depuis le début).
    AtTime(f32),
    /// Se déclenche N secondes après que l'étape précédente s'est déclenchée.
    AfterPrevious(f32),
    /// Se déclenche N secondes après l'étape nommée (par son label).
    /// Exemple : `After("spawn_boss", 5.0)` → 5s après que "spawn_boss" s'est déclenché.
    /// Si l'étape référencée n'a pas encore été déclenchée, cette étape attend.
    After(&'static str, f32),
}

/// Ce qui se passe quand une étape se déclenche.
#[derive(Clone)]
pub enum Action {
    // ─── Difficulté ─────────────────────────────────────────────
    /// Change le facteur de difficulté (vitesse astéroïdes, spawn rate).
    SetDifficulty(f32),

    // ─── Audio ──────────────────────────────────────────────────
    /// Joue un son one-shot.
    PlaySound(&'static str),
    /// Lance la musique de jeu en boucle.
    StartMusic(&'static str),
    /// Arrête la musique principale.
    StopMainMusic,

    // ─── Événements de jeu ──────────────────────────────────────
    /// Lance le countdown "3-2-1-GO".
    StartCountdown,
    /// Envoie un BoomEvent (flash + effet visuel).
    SendBoom,

    // ─── Spawning ───────────────────────────────────────────────
    /// Active le spawn des GreenUFO (intervalle en secondes).
    StartGreenUFOSpawning(f32),
    /// Désactive le spawn des GreenUFO.
    StopGreenUFOSpawning,
    /// Arrête le spawn des astéroïdes.
    StopAsteroidSpawning,
    /// Fait apparaître le boss.
    SpawnBoss,

    // ─── Environnement ──────────────────────────────────────────
    /// Démarre la décélération du fond (durée, vitesse finale).
    StartBgDeceleration { duration: f32, final_speed: f32 },
    /// Fait apparaître la planète.
    ShowPlanet,

    // ─── Log (debug) ────────────────────────────────────────────
    /// Affiche un message dans la console (debug uniquement).
    Log(&'static str),
}

/// Une étape du niveau : un déclencheur + des actions.
#[derive(Clone)]
pub struct LevelStep {
    /// Nom de l'étape (pour le debug).
    pub label: &'static str,
    /// Quand cette étape se déclenche.
    pub trigger: Trigger,
    /// Actions exécutées quand l'étape se déclenche.
    pub actions: Vec<Action>,
}

// ═══════════════════════════════════════════════════════════════════════
//  Constructeur fluide
// ═══════════════════════════════════════════════════════════════════════

impl LevelStep {
    /// Crée une étape déclenchée à un temps absolu.
    pub fn at(time: f32, label: &'static str) -> Self {
        Self {
            label,
            trigger: Trigger::AtTime(time),
            actions: Vec::new(),
        }
    }

    /// Crée une étape déclenchée N secondes après la précédente.
    pub fn after(delay: f32, label: &'static str) -> Self {
        Self {
            label,
            trigger: Trigger::AfterPrevious(delay),
            actions: Vec::new(),
        }
    }

    /// Crée une étape déclenchée N secondes après une étape nommée.
    /// Exemple : `LevelStep::after_step("spawn_boss", 3.0, "boss_music")`
    pub fn after_step(ref_label: &'static str, delay: f32, label: &'static str) -> Self {
        Self {
            label,
            trigger: Trigger::After(ref_label, delay),
            actions: Vec::new(),
        }
    }

    /// Ajoute une action à cette étape.
    pub fn with(mut self, action: Action) -> Self {
        self.actions.push(action);
        self
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Ressource : état du runner
// ═══════════════════════════════════════════════════════════════════════

/// État du déroulement du niveau.
#[derive(Resource)]
pub struct LevelRunner {
    /// Liste des étapes du niveau.
    steps: Vec<LevelStep>,
    /// Index de la prochaine étape à exécuter.
    current: usize,
    /// Temps de jeu écoulé (secondes).
    pub elapsed: f32,
    /// Temps auquel la dernière étape s'est déclenchée.
    last_trigger_time: f32,
    /// Temps de déclenchement de chaque étape, indexé par label.
    trigger_times: HashMap<&'static str, f32>,
}

impl LevelRunner {
    pub fn new(steps: Vec<LevelStep>) -> Self {
        Self {
            steps,
            current: 0,
            elapsed: 0.0,
            last_trigger_time: 0.0,
            trigger_times: HashMap::new(),
        }
    }

    /// Vérifie si toutes les étapes ont été exécutées.
    pub fn is_finished(&self) -> bool {
        self.current >= self.steps.len()
    }

    /// Retourne l'étape courante (si elle existe).
    pub fn current_step(&self) -> Option<&LevelStep> {
        self.steps.get(self.current)
    }

    /// Retourne le label de la dernière étape exécutée.
    pub fn last_label(&self) -> &str {
        if self.current == 0 {
            "(aucune)"
        } else {
            self.steps[self.current - 1].label
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Définition du niveau 1
// ═══════════════════════════════════════════════════════════════════════

pub fn build_level_1() -> Vec<LevelStep> {
    vec![
        // ─── Phase d'intro (0-7s) ───────────────────────────────
        LevelStep::at(0.0, "game_start")
            .with(Action::StartMusic("audio/gradius.ogg"))
            .with(Action::SetDifficulty(0.5))
            .with(Action::Log("Niveau 1 démarré")),

        // ─── Countdown (7-10s) ──────────────────────────────────
        LevelStep::at(7.0, "countdown")
            .with(Action::PlaySound("audio/charging.ogg"))
            .with(Action::StartCountdown),

        // ─── Phase 2 : montée en difficulté ─────────────────────
        LevelStep::at(10.0, "phase_2_start")
            .with(Action::SetDifficulty(3.5))
            .with(Action::SendBoom)
            .with(Action::StartGreenUFOSpawning(2.0)),

        LevelStep::at(14.3, "boom_1")
            .with(Action::SetDifficulty(4.5))
            .with(Action::PlaySound("audio/t_go.wav"))
            .with(Action::SendBoom),

        LevelStep::at(18.3, "boom_2")
            .with(Action::SetDifficulty(6.5))
            .with(Action::PlaySound("audio/t_go.wav"))
            .with(Action::SendBoom),

        LevelStep::at(22.6, "boom_3")
            .with(Action::SetDifficulty(7.5))
            .with(Action::PlaySound("audio/t_go.wav"))
            .with(Action::SendBoom),

        // ─── Transition vers le boss ────────────────────────────
        LevelStep::at(27.7, "pre_boss")
            .with(Action::StopAsteroidSpawning)
            .with(Action::StopGreenUFOSpawning)
            .with(Action::StartBgDeceleration {
                duration: 9.0,
                final_speed: 30.0,
            }),

        LevelStep::at(28.0, "planet_appear")
            .with(Action::ShowPlanet),

        LevelStep::at(35.8, "boss_spawn")
            .with(Action::SpawnBoss)
            .with(Action::StopMainMusic)
            .with(Action::Log("Boss spawné !")),

        // ─── Événements chaînés au boss ─────────────────────────
        // Exemple de Trigger::After : musique du boss 5s après son spawn
        LevelStep::after_step("boss_spawn", 5.0, "boss_music")
            .with(Action::StartMusic("audio/boss.ogg"))
            .with(Action::Log("Musique du boss lancée")),

        // ─── Les événements suivants sont gérés par boss.rs ─────
        // Le boss gère lui-même sa séquence interne :
        //   Entering → Flexing → Idle → musique → Active
        // car ces transitions dépendent de l'état du boss,
        // pas du temps absolu.
    ]
}

// ═══════════════════════════════════════════════════════════════════════
//  Systèmes
// ═══════════════════════════════════════════════════════════════════════

fn setup_level(mut commands: Commands) {
    let steps = build_level_1();
    commands.insert_resource(LevelRunner::new(steps));
}

fn run_level(
    mut commands: Commands,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    mut runner: ResMut<LevelRunner>,
    mut boom_events: EventWriter<BoomEvent>,
    mut countdown_events: EventWriter<crate::countdown::CountdownEvent>,
) {
    runner.elapsed += time.delta_seconds();

    // Exécuter toutes les étapes dont le déclencheur est atteint
    loop {
        if runner.is_finished() {
            break;
        }

        let step = runner.steps[runner.current].clone();
        let should_trigger = match &step.trigger {
            Trigger::AtTime(t) => runner.elapsed >= *t,
            Trigger::AfterPrevious(delay) => {
                runner.elapsed >= runner.last_trigger_time + delay
            }
            Trigger::After(label, delay) => {
                if let Some(&ref_time) = runner.trigger_times.get(label) {
                    runner.elapsed >= ref_time + delay
                } else {
                    false // L'étape référencée n'a pas encore été déclenchée
                }
            }
        };

        if !should_trigger {
            break;
        }

        // Exécuter les actions de cette étape
        info!("[Level] {:>6.1}s — {}", runner.elapsed, step.label);

        for action in &step.actions {
            execute_action(
                action,
                &mut commands,
                &asset_server,
                &mut boom_events,
                &mut countdown_events,
            );
        }

        // Enregistrer le temps de déclenchement pour ce label
        let elapsed = runner.elapsed;
        runner.trigger_times.insert(step.label, elapsed);
        runner.last_trigger_time = elapsed;
        runner.current += 1;
    }
}

fn execute_action(
    action: &Action,
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    boom_events: &mut EventWriter<BoomEvent>,
    countdown_events: &mut EventWriter<crate::countdown::CountdownEvent>,
) {
    match action {
        Action::SetDifficulty(_factor) => {
            // TODO: écrire dans Difficulty.factor
            // Pour l'instant, difficulty.rs gère encore cela
        }
        Action::PlaySound(path) => {
            commands.spawn(AudioBundle {
                source: asset_server.load(*path),
                settings: PlaybackSettings::DESPAWN,
            });
        }
        Action::StartMusic(path) => {
            commands.spawn((
                AudioBundle {
                    source: asset_server.load(*path),
                    settings: PlaybackSettings {
                        mode: bevy::audio::PlaybackMode::Once,
                        ..default()
                    },
                },
                crate::MusicMain,
            ));
        }
        Action::StopMainMusic => {
            // TODO: despawn MusicMain entities
        }
        Action::StartCountdown => {
            countdown_events.send(crate::countdown::CountdownEvent);
        }
        Action::SendBoom => {
            boom_events.send(BoomEvent);
        }
        Action::StartGreenUFOSpawning(_interval) => {
            // TODO: activer le spawner GreenUFO avec l'intervalle donné
        }
        Action::StopGreenUFOSpawning => {
            // TODO: désactiver le spawner GreenUFO
        }
        Action::StopAsteroidSpawning => {
            // TODO: mettre spawning_stopped = true dans Difficulty
        }
        Action::SpawnBoss => {
            // TODO: déclencher le spawn du boss
        }
        Action::StartBgDeceleration { duration: _, final_speed: _ } => {
            // TODO: configurer la décélération du background
        }
        Action::ShowPlanet => {
            // TODO: déclencher l'apparition de la planète
        }
        Action::Log(msg) => {
            info!("[Level] {}", msg);
        }
    }
}
