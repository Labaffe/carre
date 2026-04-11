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

use crate::difficulty::{BoomEvent, Difficulty, SpawnPosition};
use crate::pause::not_paused;
use crate::state::GameState;
use bevy::prelude::*;

/// Événement permettant à n'importe quel système d'injecter des actions
/// dans le pipeline du niveau. Les actions sont exécutées immédiatement
/// par le système `process_level_action_events`.
///
/// Exemple depuis un système boss :
/// ```ignore
/// level_events.send(LevelActionEvent(vec![
///     Action::SpawnEnemy("green_ufo", 8),
///     Action::PlaySound("audio/alert.ogg"),
/// ]));
/// ```
#[derive(Event)]
pub struct LevelActionEvent(pub Vec<Action>);

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<LevelActionEvent>()
            .add_systems(OnEnter(GameState::Playing), setup_level)
            .add_systems(
                Update,
                (run_level, process_level_action_events)
                    .run_if(in_state(GameState::Playing))
                    .run_if(not_paused),
            )
            .add_systems(OnExit(GameState::Playing), cleanup_level);
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
    /// Spawn N ennemis d'un type donné à une position donnée.
    /// Ex: `SpawnEnemy("boss", 1, SpawnPosition::At(0.0, 50.0))`.
    SpawnEnemy(&'static str, usize, SpawnPosition),
    /// Active le spawn continu d'un type d'ennemi.
    /// (nom, quantité par vague, intervalle en secondes, position)
    /// Ex: `StartSpawning("green_ufo", 4, 5.0, SpawnPosition::Top)`.
    StartSpawning(&'static str, usize, f32, SpawnPosition),
    /// Désactive le spawn continu d'un type d'ennemi.
    StopSpawning(&'static str),

    // ─── Environnement ──────────────────────────────────────────
    /// Démarre la décélération du fond (durée, vitesse finale).
    StartBgDeceleration { duration: f32, final_speed: f32 },
    /// Fait apparaître la planète.
    ShowPlanet,

    // ─── Progression ─────────────────────────────────────────────
    /// Marque le niveau comme terminé (déclenche le countdown → outro).
    MarkLevelComplete,

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

    /// Retourne toutes les étapes du niveau.
    pub fn steps(&self) -> &[LevelStep] {
        &self.steps
    }

    /// Retourne l'index de la prochaine étape à exécuter.
    pub fn current_index(&self) -> usize {
        self.current
    }

    /// Retourne le temps de déclenchement d'une étape par son label (si déjà déclenchée).
    pub fn trigger_time(&self, label: &str) -> Option<f32> {
        self.trigger_times.get(label).copied()
    }

    /// Avance le runner jusqu'au label donné (inclus), en marquant toutes
    /// les étapes intermédiaires comme exécutées au temps `at_time`.
    /// Retourne les actions de toutes les étapes sautées + l'étape cible.
    ///
    /// Si le label a déjà été exécuté, met juste à jour `elapsed` et
    /// retourne un vecteur vide (pas de double exécution).
    pub fn skip_to(&mut self, label: &str, at_time: f32) -> Vec<Vec<Action>> {
        // Label déjà exécuté → juste mettre à jour le temps
        if self.trigger_times.contains_key(label) {
            self.elapsed = at_time.max(self.elapsed);
            return Vec::new();
        }

        let mut all_actions = Vec::new();
        self.elapsed = at_time;

        loop {
            if self.current >= self.steps.len() {
                break;
            }

            let step = &self.steps[self.current];
            let step_label = step.label;
            let actions = step.actions.clone();

            self.trigger_times.insert(step_label, at_time);
            self.last_trigger_time = at_time;
            all_actions.push(actions);
            self.current += 1;

            if step_label == label {
                break;
            }
        }

        all_actions
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Affichage court des actions (pour le debug)
// ═══════════════════════════════════════════════════════════════════════

impl Action {
    /// Indique si cette action doit être rejouée lors d'un skip debug (F2/F3).
    /// Les actions d'état (difficulté, spawners, background) sont rejouées.
    /// Les actions cosmétiques (sons, musique, booms, spawns) sont ignorées.
    pub fn should_replay_on_skip(&self) -> bool {
        matches!(
            self,
            Action::SetDifficulty(_)
                | Action::StopMainMusic
                | Action::StopSpawning(_)
                | Action::StartBgDeceleration { .. }
                | Action::ShowPlanet
        )
    }

    /// Retourne un nom court de l'action pour l'overlay debug.
    pub fn short_name(&self) -> String {
        match self {
            Action::SetDifficulty(f) => format!("Diff({})", f),
            Action::PlaySound(p) => {
                let name = p.rsplit('/').next().unwrap_or(p);
                format!("Sound({})", name)
            }
            Action::StartMusic(p) => {
                let name = p.rsplit('/').next().unwrap_or(p);
                format!("Music({})", name)
            }
            Action::StopMainMusic => "StopMusic".to_string(),
            Action::StartCountdown => "Countdown".to_string(),
            Action::SendBoom => "Boom".to_string(),
            Action::SpawnEnemy(name, count, pos) => {
                let pos_str = match pos {
                    SpawnPosition::Top => "",
                    SpawnPosition::Bottom => " ↓",
                    SpawnPosition::Left => " ←",
                    SpawnPosition::Right => " →",
                    SpawnPosition::At(x, y) => return format!("Spawn({}×{} @{:.0},{:.0})", count, name, x, y),
                };
                if *count == 1 {
                    format!("Spawn({}{})", name, pos_str)
                } else {
                    format!("Spawn({}×{}{})", count, name, pos_str)
                }
            }
            Action::StartSpawning(name, count, interval, pos) => {
                let pos_str = match pos {
                    SpawnPosition::Top => "",
                    SpawnPosition::Bottom => " ↓",
                    SpawnPosition::Left => " ←",
                    SpawnPosition::Right => " →",
                    SpawnPosition::At(x, y) => return format!("Start({}×{},{}s @{:.0},{:.0})", count, name, interval, x, y),
                };
                format!("Start({}×{},{}s{})", count, name, interval, pos_str)
            }
            Action::StopSpawning(name) => format!("Stop({})", name),
            Action::StartBgDeceleration {
                duration,
                final_speed,
            } => {
                format!("BgDecel({}s,{})", duration, final_speed)
            }
            Action::ShowPlanet => "Planet".to_string(),
            Action::MarkLevelComplete => "LevelComplete".to_string(),
            Action::Log(msg) => format!("Log({})", msg),
        }
    }
}

impl Trigger {
    /// Retourne une description courte du trigger pour l'overlay debug.
    pub fn short_desc(&self) -> String {
        match self {
            Trigger::AtTime(t) => format!("@ {:.1}s", t),
            Trigger::AfterPrevious(d) => format!("+{:.1}s (prev)", d),
            Trigger::After(label, d) => format!("+{:.1}s -> {}", d, label),
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
            .with(Action::StartSpawning("asteroid", 1, 1.0, SpawnPosition::Top))
            .with(Action::Log("Niveau 1 démarré")),
        // ─── Countdown (7-10s) ──────────────────────────────────
        LevelStep::at(7.0, "countdown")
            .with(Action::PlaySound("audio/charging.ogg"))
            .with(Action::StartCountdown),
        // Note : le countdown envoie un BoomEvent au "GO!" (10s)

        // ─── Phase 2 : montée en difficulté ─────────────────────
        LevelStep::at(10.0, "phase_2_start")
            .with(Action::SetDifficulty(3.5))
            .with(Action::StartSpawning("green_ufo", 2, 4.0, SpawnPosition::Top)),
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
            .with(Action::StopSpawning("asteroid"))
            .with(Action::StopSpawning("green_ufo"))
            .with(Action::StartBgDeceleration {
                duration: 9.0,
                final_speed: 30.0,
            }),
        LevelStep::at(28.0, "planet_appear").with(Action::ShowPlanet),
        LevelStep::at(35.8, "boss_spawn")
            .with(Action::SpawnEnemy("boss", 1, SpawnPosition::At(0.0, 50.0)))
            .with(Action::StopMainMusic)
            .with(Action::Log("Boss 1 spawné !")),
        // ─── Le boss gère sa propre séquence interne ──────────
        //   Entering → Flexing → Idle → Active
        // La musique boss (boss.ogg) est lancée quand le boss
        // atteint Idle, et s'arrête à sa mort.
    ]
}

// ═══════════════════════════════════════════════════════════════════════
//  Définition du niveau 2
// ═══════════════════════════════════════════════════════════════════════

pub fn build_level_2() -> Vec<LevelStep> {
    vec![
        LevelStep::at(0.0, "game_start")
            .with(Action::Log("Niveau 2 démarré")),
        LevelStep::at(2.0, "level_complete")
            .with(Action::MarkLevelComplete)
            .with(Action::Log("Niveau 2 terminé")),
    ]
}

// ═══════════════════════════════════════════════════════════════════════
//  Systèmes
// ═══════════════════════════════════════════════════════════════════════

fn setup_level(mut commands: Commands, progress: Res<crate::game::GameProgress>) {
    let steps = match progress.current_level {
        1 => build_level_1(),
        2 => build_level_2(),
        _ => build_level_1(), // fallback
    };
    commands.insert_resource(LevelRunner::new(steps));
}

fn run_level(
    mut commands: Commands,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    runner: Option<ResMut<LevelRunner>>,
    mut difficulty: ResMut<Difficulty>,
    mut boom_events: EventWriter<BoomEvent>,
    mut countdown_events: EventWriter<crate::countdown::CountdownEvent>,
    music_q: Query<Entity, With<crate::MusicMain>>,
) {
    let Some(mut runner) = runner else { return };
    runner.elapsed += time.delta_seconds();

    // Exécuter toutes les étapes dont le déclencheur est atteint
    loop {
        if runner.is_finished() {
            break;
        }

        let step = runner.steps[runner.current].clone();
        let should_trigger = match &step.trigger {
            Trigger::AtTime(t) => runner.elapsed >= *t,
            Trigger::AfterPrevious(delay) => runner.elapsed >= runner.last_trigger_time + delay,
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
                &mut difficulty,
                &music_q,
            );
        }

        // Enregistrer le temps de déclenchement pour ce label
        let elapsed = runner.elapsed;
        runner.trigger_times.insert(step.label, elapsed);
        runner.last_trigger_time = elapsed;
        runner.current += 1;
    }
}

pub(crate) fn execute_action(
    action: &Action,
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    boom_events: &mut EventWriter<BoomEvent>,
    countdown_events: &mut EventWriter<crate::countdown::CountdownEvent>,
    difficulty: &mut ResMut<Difficulty>,
    music_q: &Query<Entity, With<crate::MusicMain>>,
) {
    match action {
        Action::SetDifficulty(factor) => {
            difficulty.factor = *factor;
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
            for entity in music_q.iter() {
                if let Some(e) = commands.get_entity(entity) { e.despawn_recursive(); }
            }
        }
        Action::StartCountdown => {
            countdown_events.send(crate::countdown::CountdownEvent);
        }
        Action::SendBoom => {
            boom_events.send(BoomEvent);
        }
        Action::SpawnEnemy(name, count, pos) => {
            difficulty.spawn_requests.push((name, *count, *pos));
        }
        Action::StartSpawning(name, count, interval, pos) => {
            difficulty.active_spawners.insert(name, (*count, *interval, *pos));
        }
        Action::StopSpawning(name) => {
            difficulty.active_spawners.remove(name);
        }
        Action::StartBgDeceleration {
            duration,
            final_speed,
        } => {
            difficulty.bg_decel_start_elapsed = Some(difficulty.elapsed);
            difficulty.bg_decel_duration = *duration;
            difficulty.bg_decel_final_speed = *final_speed;
        }
        Action::ShowPlanet => {
            difficulty.planet_appear_elapsed = Some(difficulty.elapsed);
        }
        Action::MarkLevelComplete => {
            difficulty.boss_spawned = true;
        }
        Action::Log(msg) => {
            info!("[Level] {}", msg);
        }
    }
}

/// Consomme les `LevelActionEvent` envoyés par d'autres systèmes (boss, ennemis…)
/// et exécute leurs actions via le même pipeline que la timeline.
fn process_level_action_events(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut events: EventReader<LevelActionEvent>,
    mut difficulty: ResMut<Difficulty>,
    mut boom_events: EventWriter<BoomEvent>,
    mut countdown_events: EventWriter<crate::countdown::CountdownEvent>,
    music_q: Query<Entity, With<crate::MusicMain>>,
) {
    for event in events.read() {
        for action in &event.0 {
            info!("[Level] (event) {}", action.short_name());
            execute_action(
                action,
                &mut commands,
                &asset_server,
                &mut boom_events,
                &mut countdown_events,
                &mut difficulty,
                &music_q,
            );
        }
    }
}

fn cleanup_level(mut commands: Commands) {
    commands.remove_resource::<LevelRunner>();
}
