//! Niveau "Chaos" : système de **paliers à budget de points**.
//!
//! Chaque palier (tier) a un budget aléatoire qui grandit avec le tier.
//! Pendant la durée du palier, des ennemis sont spawnés à intervalles
//! réguliers. À chaque tentative de spawn :
//! - filtre les ennemis affordables (`cost ≤ budget restant`)
//! - tirage pondéré (`weight`) parmi les affordables
//! - dépense le `cost` de l'ennemi choisi sur le budget
//!
//! Le palier se termine quand le budget est épuisé OU sa durée écoulée.
//! Suit un cooldown court, puis tier++ et nouveau budget plus généreux.
//!
//! Paramétrage :
//! - `default_tunings()` : cost/weight par ennemi
//! - `tier_budget()` : progression des budgets par tier
//! - Constantes `SPAWN_INTERVAL`, `TIER_DURATION`, `COOLDOWN_DURATION`
//!
//! Pour personnaliser au setup :
//! ```ignore
//! ChaosConfig::default()
//!     .exclude("boss")               // pas de boss
//!     .with_position(SpawnPosition::Top)
//! ```

use crate::game_manager::difficulty::{Difficulty, SpawnPosition};
use bevy::prelude::*;

/// Type de spawn associé à un tuning : ennemi unique standard, ou wave
/// composée (cf. `SimpleUfoWaveSpawner` qui spawn N ufos en queue le long
/// d'un chemin Bézier).
#[derive(Clone, Copy)]
pub enum SpawnKind {
    Single,
    SimpleUfoWave { count: usize, interval: f32 },
}

/// Réglage par ennemi : `cost` consommé au spawn, `weight` pondère la prob
/// de sélection parmi les ennemis affordables au moment du spawn.
/// `kind` détermine la mécanique de spawn (single ou wave Bézier).
#[derive(Clone)]
pub struct EnemyTuning {
    pub name: &'static str,
    pub cost: u32,
    pub weight: u32,
    pub kind: SpawnKind,
}

/// Intervalle entre 2 tentatives de spawn — décroît exponentiellement avec
/// le tier pour augmenter la vitesse de dépense du budget. Tier 1 = 0.7s,
/// plafonné à `SPAWN_INTERVAL_MIN` aux tiers élevés.
fn spawn_interval(tier: u32) -> f32 {
    let base = 0.7;
    let decay: f32 = 0.88; // tier 5 ≈ 0.42s, tier 10 ≈ 0.22s
    (base * decay.powi((tier as i32 - 1).max(0))).max(SPAWN_INTERVAL_MIN)
}
const SPAWN_INTERVAL_MIN: f32 = 0.2;
/// Durée max d'un palier avant transition vers cooldown (secondes).
const TIER_DURATION: f32 = 15.0;
/// Pause entre 2 paliers (secondes).
const COOLDOWN_DURATION: f32 = 4.0;

/// Renvoie le range `(min, max)` du budget pour le tier donné. Le tier 1 est
/// le début du niveau ; les budgets grandissent vite pour rendre les paliers
/// chaotiques. À partir du tier 7, le budget grimpe linéairement
/// (tier×40 .. tier×60).
fn tier_budget(tier: u32) -> (u32, u32) {
    match tier {
        0 | 1 => (15, 35),
        2 => (30, 70),
        3 => (50, 120),
        4 => (80, 180),
        5 => (120, 250),
        6 => (180, 350),
        _ => (tier * 40, tier * 60),
    }
}

/// Tunings par défaut.
/// - `asteroid` (cost 1, weight 50) : remplisseur ultra-courant
/// - `kamikaze` (cost 5, weight 25) : threat mid, fréquent dès tier 1
/// - `green_ufo` / `mine` (cost 3-4, weight 12 chacun) : variété
/// - `simple_ufo_wave` (cost 8, weight 8) : wave Bézier de 5 UFOs ; le nom
///   logique du tuning n'est pas un EnemyBuilder — c'est le `SpawnKind` qui
///   gouverne le spawn (un `SimpleUfoWaveSpawner` est inséré directement).
/// - `octopus` (cost 15, weight 5) : ne peut spawn qu'à partir de tier 3
/// - `octopus_green` (cost 18, weight 4) : variante intangible-rush, rare
/// - `vaisseau` (cost 25, weight 3) : groupe parent + 4 tourelles, rare
/// - `boss` (cost 60, weight 1) : ne peut spawn qu'à partir de tier 5, très rare
fn default_tunings() -> Vec<EnemyTuning> {
    vec![
        EnemyTuning { name: "asteroid",           cost: 1,  weight: 50, kind: SpawnKind::Single },
        EnemyTuning { name: "kamikaze",           cost: 5,  weight: 25, kind: SpawnKind::Single },
        EnemyTuning { name: "green_ufo",          cost: 4,  weight: 12, kind: SpawnKind::Single },
        EnemyTuning { name: "mine",               cost: 3,  weight: 12, kind: SpawnKind::Single },
        EnemyTuning { name: "simple_ufo_shooter", cost: 6,  weight: 10, kind: SpawnKind::Single },
        EnemyTuning { name: "simple_ufo_wave",    cost: 8,  weight: 8,  kind: SpawnKind::SimpleUfoWave { count: 5, interval: 0.2 } },
        EnemyTuning { name: "octopus",            cost: 15, weight: 5,  kind: SpawnKind::Single },
        EnemyTuning { name: "octopus_green",      cost: 18, weight: 4,  kind: SpawnKind::Single },
        EnemyTuning { name: "vaisseau",           cost: 25, weight: 3,  kind: SpawnKind::Single },
        EnemyTuning { name: "boss",               cost: 60, weight: 1,  kind: SpawnKind::Single },
    ]
}

/// État courant du chaos spawner — machine binaire : palier actif (spawn
/// d'ennemis) ↔ cooldown entre 2 paliers.
#[derive(Clone)]
pub enum ChaosState {
    /// Spawn en cours, budget restant à dépenser.
    Active {
        budget: i32,
        elapsed: f32,
        duration: f32,
        spawn_timer: Timer,
    },
    /// Pause avant le palier suivant.
    Cooldown { timer: Timer },
}

#[derive(Resource, Clone)]
pub struct ChaosConfig {
    pub enemies: Vec<EnemyTuning>,
    pub spawn_position: SpawnPosition,
    pub tier: u32,
    pub state: ChaosState,
}

impl Default for ChaosConfig {
    fn default() -> Self {
        Self {
            enemies: default_tunings(),
            spawn_position: SpawnPosition::Top,
            tier: 0,
            // Cooldown initial à 0s : le 1er tick transitionne immédiatement
            // vers tier 1 actif (premier spawn ~SPAWN_INTERVAL plus tard).
            state: ChaosState::Cooldown {
                timer: Timer::from_seconds(0.0, TimerMode::Once),
            },
        }
    }
}

impl ChaosConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_position(mut self, pos: SpawnPosition) -> Self {
        self.spawn_position = pos;
        self
    }

    /// Retire un ennemi des tunings (si présent).
    pub fn exclude(mut self, enemy: &'static str) -> Self {
        self.enemies.retain(|e| e.name != enemy);
        self
    }

    /// Ré-ajoute un ennemi avec ses cost/weight par défaut (si pas déjà présent).
    pub fn include(mut self, enemy: &'static str) -> Self {
        if !self.enemies.iter().any(|e| e.name == enemy) {
            if let Some(default) = default_tunings().into_iter().find(|e| e.name == enemy) {
                self.enemies.push(default);
            }
        }
        self
    }
}

/// Système principal : pilote la machine à état et pousse des SpawnRequests
/// dans `difficulty.spawn_requests` selon les tunings. Pour les `SpawnKind`
/// composés (ex: `SimpleUfoWave`), spawn directement le composant contrôleur
/// au lieu de passer par la file de spawn standard.
pub fn chaos_spawner_system(
    time: Res<Time>,
    mut chaos: ResMut<ChaosConfig>,
    mut difficulty: ResMut<Difficulty>,
    mut commands: Commands,
) {
    let dt = time.delta();
    let dt_secs = time.delta_secs();
    let chaos = &mut *chaos;
    let enemies_snapshot = chaos.enemies.clone();
    let spawn_position = chaos.spawn_position;

    let next_state: Option<ChaosState> = match &mut chaos.state {
        ChaosState::Active { budget, elapsed, duration, spawn_timer } => {
            *elapsed += dt_secs;
            // Tier ends : budget épuisé OU durée écoulée.
            if *elapsed >= *duration || *budget <= 0 {
                Some(ChaosState::Cooldown {
                    timer: Timer::from_seconds(COOLDOWN_DURATION, TimerMode::Once),
                })
            } else {
                spawn_timer.tick(dt);
                if spawn_timer.just_finished() {
                    if let Some(pick) = pick_enemy(&enemies_snapshot, *budget) {
                        match pick.kind {
                            SpawnKind::Single => {
                                difficulty
                                    .spawn_requests
                                    .push((pick.name, 1, spawn_position));
                            }
                            SpawnKind::SimpleUfoWave { count, interval } => {
                                commands.spawn(
                                    crate::enemy::simple_ufo::SimpleUfoWaveSpawner::new(
                                        count, interval,
                                    ),
                                );
                            }
                        }
                        *budget -= pick.cost as i32;
                    }
                    // Si aucun ennemi affordable, on attend l'expiration du
                    // tier (durée) — ça arrive si budget < cost min.
                }
                None
            }
        }
        ChaosState::Cooldown { timer } => {
            timer.tick(dt);
            if timer.is_finished() {
                let new_tier = chaos.tier + 1;
                let (min, max) = tier_budget(new_tier);
                let budget = if max > min {
                    fastrand::u32(min..=max) as i32
                } else {
                    max as i32
                };
                Some(ChaosState::Active {
                    budget,
                    elapsed: 0.0,
                    duration: TIER_DURATION,
                    spawn_timer: Timer::from_seconds(
                        spawn_interval(new_tier),
                        TimerMode::Repeating,
                    ),
                })
            } else {
                None
            }
        }
    };

    if let Some(next) = next_state {
        if matches!(next, ChaosState::Active { .. }) {
            chaos.tier += 1;
        }
        chaos.state = next;
    }
}

// ─── Playlist musicale du chaos ─────────────────────────────────────

/// Pistes musicales du mode chaos, jouées en boucle l'une après l'autre.
const CHAOS_PLAYLIST: &[&str] = &[
    "audio/music/gradius.ogg",
    "audio/music/boss.ogg",
];

/// État de la playlist : index de la **prochaine** piste à spawner.
/// Inséré sur le niveau Chaos par `setup_level` ; le système
/// `chaos_music_system` ne tourne que tant que cette ressource existe.
#[derive(Resource, Default)]
pub struct ChaosMusicState {
    pub next_index: usize,
}

/// Pilote la playlist du chaos : si aucune `MusicMain` n'est en cours ou si
/// l'`AudioSink` de l'actuelle est vide (terminée), despawn l'ancienne et
/// spawn la piste suivante en avançant `next_index`.
pub fn chaos_music_system(
    mut commands: Commands,
    mut state: ResMut<ChaosMusicState>,
    asset_server: Res<AssetServer>,
    music_q: Query<(Entity, Option<&bevy::audio::AudioSink>), With<crate::MusicMain>>,
) {
    let mut needs_spawn = false;
    let mut finished_entity: Option<Entity> = None;

    if let Some((entity, sink_opt)) = music_q.iter().next() {
        // Si l'AudioSink n'est pas encore attaché (frame 1 après spawn),
        // sink_opt = None → on attend.
        if let Some(sink) = sink_opt {
            if sink.empty() {
                needs_spawn = true;
                finished_entity = Some(entity);
            }
        }
    } else {
        // Aucune entité de musique : 1er démarrage du niveau.
        needs_spawn = true;
    }

    if !needs_spawn {
        return;
    }

    if let Some(entity) = finished_entity {
        if let Ok(mut e) = commands.get_entity(entity) {
            e.try_despawn();
        }
    }

    if CHAOS_PLAYLIST.is_empty() {
        return;
    }
    let path = CHAOS_PLAYLIST[state.next_index];
    commands.spawn((
        AudioPlayer::new(asset_server.load(path)),
        PlaybackSettings {
            mode: bevy::audio::PlaybackMode::Once,
            ..default()
        },
        crate::MusicMain,
    ));
    state.next_index = (state.next_index + 1) % CHAOS_PLAYLIST.len();
}

// ─── UI compteur de chaos level ─────────────────────────────────────

/// Marker sur le `Text` qui affiche le tier courant du chaos.
#[derive(Component)]
#[require(crate::GameplayEntity)]
pub struct ChaosLevelUI;

/// Spawn le texte du compteur à `OnEnter(GameState::Playing)`. Ne tourne
/// qu'avec `run_if(resource_exists::<ChaosConfig>)` côté plugin → présent
/// uniquement sur le niveau Chaos.
pub fn setup_chaos_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    existing: Query<Entity, With<ChaosLevelUI>>,
) {
    // Idempotent : ne rien faire si déjà spawné (au cas où le system tourne
    // 2× pour une raison de scheduling).
    if existing.iter().next().is_some() {
        return;
    }
    let font = asset_server.load("fonts/PressStart2P-Regular.ttf");
    commands.spawn((
        Text::new("CHAOS LV 0"),
        TextFont {
            font,
            font_size: 24.0,
            ..default()
        },
        // Orange agressif pour signaler le mode chaos.
        TextColor(Color::srgba(1.0, 0.5, 0.15, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(20.0),
            left: Val::Percent(50.0),
            // Marge à gauche pour centrer-ish via translate (~120px = demi
            // largeur approximative du texte le plus long).
            margin: UiRect::left(Val::Px(-100.0)),
            ..default()
        },
        ChaosLevelUI,
    ));
}

/// Met à jour le texte du compteur chaque frame avec le tier courant et
/// l'état (budget restant si actif, countdown si cooldown).
pub fn update_chaos_ui(
    chaos: Res<ChaosConfig>,
    mut text_q: Query<&mut Text, With<ChaosLevelUI>>,
) {
    let Ok(mut text) = text_q.single_mut() else { return };
    let status = match &chaos.state {
        ChaosState::Active { budget, .. } => {
            format!("CHAOS LV {}  [{} pts]", chaos.tier, budget.max(&0))
        }
        ChaosState::Cooldown { timer } => {
            let remaining = (timer.duration().as_secs_f32() - timer.elapsed_secs()).max(0.0);
            format!("CHAOS LV {} in {:.1}s", chaos.tier + 1, remaining)
        }
    };
    **text = status;
}

// `cleanup_chaos_ui` retiré — cleanup auto via `cleanup_playing` (main.rs)
// grâce à `#[require(GameplayEntity)]` sur `ChaosLevelUI`.

// ─── Logique de spawn ───────────────────────────────────────────────

/// Tirage pondéré d'un ennemi parmi ceux affordables. Renvoie `None` si
/// le budget est inférieur au plus petit cost (=> on attend la fin du tier).
fn pick_enemy<'a>(enemies: &'a [EnemyTuning], budget: i32) -> Option<&'a EnemyTuning> {
    let affordable: Vec<&EnemyTuning> = enemies
        .iter()
        .filter(|e| (e.cost as i32) <= budget)
        .collect();
    if affordable.is_empty() {
        return None;
    }
    let total_weight: u32 = affordable.iter().map(|e| e.weight).sum();
    if total_weight == 0 {
        return None;
    }
    let mut roll = fastrand::u32(0..total_weight);
    for e in affordable {
        if roll < e.weight {
            return Some(e);
        }
        roll -= e.weight;
    }
    None
}
