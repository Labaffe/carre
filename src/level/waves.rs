//! Niveau "Vagues" — groupes cohérents d'ennemis, gated par kills.
//!
//! ## Cycle
//! 1. **Idle** (settle court, ~1.5s) — respiration avant la prochaine vague.
//! 2. **Spawning** — déverse les membres du template selectionné, 1 par
//!    `SPAWN_INTERVAL`. Chaque ennemi spawn reçoit un marker `WaveMember`
//!    via le système `mark_wave_members`.
//! 3. **Waiting** — tous les membres ont été spawn ; on attend que TOUS les
//!    `WaveMember` aient été tués (count = 0) pour reboucler en Idle.
//!
//! La progression est gouvernée par [`WAVE_TEMPLATES`] (groupes
//! pré-équilibrés). Les premières vagues sont volontairement légères pour
//! laisser le joueur respirer avec son arme de base. Au-delà du dernier
//! template, on pioche aléatoirement parmi les templates "hard" pour cycler.

use crate::enemy::enemy::Enemy;
use crate::enemy::simple_ufo::SimpleUfoWaveSpawner;
use crate::game_manager::difficulty::{Difficulty, SpawnPosition};
use crate::level::enemy_pool::{default_tunings, spawn_picked, EnemyTuning};
use bevy::prelude::*;

/// Settle court entre 2 vagues (juste pour respirer après le dernier kill).
const SETTLE_DURATION: f32 = 1.5;
/// Intervalle entre 2 spawns au sein d'une vague (effet "queue").
const SPAWN_INTERVAL: f32 = 0.35;
/// Largeur de la fenêtre de poids autour du `wave_weight_target(n)` :
/// la sélection pioche aléatoirement parmi tous les templates dont le poids
/// total tombe dans `[target - WINDOW/2, target + WINDOW/2]`. Plus la
/// fenêtre est large, plus la variété par vague est grande.
const WEIGHT_WINDOW: u32 = 14;
/// Poids "cible" de la vague `n`. Croissance linéaire douce : départ tendre
/// pour laisser respirer le joueur avec son arme de base, montée progressive.
fn wave_weight_target(n: u32) -> u32 {
    4 + n * 3 // n=1 → 7, n=5 → 19, n=10 → 34, n=20 → 64
}

/// Recette d'une vague : nom (UI/debug) + liste `(enemy_name, count)`.
/// Les noms doivent correspondre à des `EnemyTuning.name` du pool.
pub struct WaveTemplate {
    pub name: &'static str,
    pub members: &'static [(&'static str, u32)],
}

/// Catalogue de templates. **Ordre indifférent** : la sélection se fait par
/// fenêtre de poids glissante (`wave_weight_target`), pas par index. Pour
/// rééquilibrer le rythme, ajuster `wave_weight_target` et `WEIGHT_WINDOW`
/// en haut du fichier. Pour ajouter du contenu : juste ajouter un
/// `WaveTemplate` ici — il sera disponible dès que `wave_number` atteint
/// son poids cible. Plusieurs templates de poids ~égal forment des "tiers
/// équivalents" piochés au hasard.
const WAVE_TEMPLATES: &[WaveTemplate] = &[
    // ─── Très léger (poids 1-5) ──────────────────────────
    WaveTemplate { name: "Astéroïde solo", members: &[("asteroid", 1)] }, // 1
    WaveTemplate { name: "Petite pluie", members: &[("asteroid", 3)] }, // 3
    WaveTemplate { name: "Patrouilleur seul", members: &[("green_ufo", 1)] }, // 4
    WaveTemplate { name: "Mine isolée", members: &[("mine", 1), ("asteroid", 1)] }, // 4

    // ─── Léger (6-10) ────────────────────────────────────
    WaveTemplate { name: "Reconnaissance", members: &[("asteroid", 3), ("green_ufo", 1)] }, // 7
    WaveTemplate { name: "Patrouille verte", members: &[("green_ufo", 2)] }, // 8
    WaveTemplate { name: "Horde UFO", members: &[("simple_ufo_wave", 1)] }, // 8
    WaveTemplate { name: "Mines & asteroids", members: &[("mine", 2), ("asteroid", 3)] }, // 9
    WaveTemplate { name: "Tireur léger", members: &[("simple_ufo_shooter", 1), ("asteroid", 3)] }, // 9
    WaveTemplate { name: "Mine + kamikaze", members: &[("mine", 1), ("kamikaze", 1), ("asteroid", 2)] }, // 10

    // ─── Médium-léger (11-15) ────────────────────────────
    WaveTemplate { name: "Champ de mines", members: &[("mine", 3), ("asteroid", 2)] }, // 11
    WaveTemplate { name: "Horde + asteroids", members: &[("simple_ufo_wave", 1), ("asteroid", 3)] }, // 11
    WaveTemplate { name: "Charge kamikaze", members: &[("kamikaze", 2), ("asteroid", 2)] }, // 12
    WaveTemplate { name: "Duo verts", members: &[("green_ufo", 2), ("simple_ufo_shooter", 1)] }, // 14
    WaveTemplate { name: "Mix léger", members: &[("asteroid", 4), ("kamikaze", 1), ("green_ufo", 1)] }, // 13
    WaveTemplate { name: "Octopus seul", members: &[("octopus", 1)] }, // 15

    // ─── Médium (16-22) ──────────────────────────────────
    WaveTemplate { name: "Double horde", members: &[("simple_ufo_wave", 2)] }, // 16
    WaveTemplate { name: "Horde + kamikazes", members: &[("simple_ufo_wave", 1), ("kamikaze", 2)] }, // 18
    WaveTemplate { name: "Trio mixte", members: &[("mine", 2), ("green_ufo", 2), ("kamikaze", 1)] }, // 19
    WaveTemplate { name: "Octopus vert", members: &[("octopus_green", 1), ("asteroid", 3)] }, // 21
    WaveTemplate { name: "Duo de tireurs", members: &[("simple_ufo_shooter", 2), ("green_ufo", 2)] }, // 20
    WaveTemplate { name: "Hordes + tireur", members: &[("simple_ufo_wave", 2), ("simple_ufo_shooter", 1)] }, // 22

    // ─── Médium-lourd (23-30) ────────────────────────────
    WaveTemplate { name: "Bombardement", members: &[("mine", 3), ("kamikaze", 3)] }, // 24
    WaveTemplate { name: "Triple horde", members: &[("simple_ufo_wave", 3)] }, // 24
    WaveTemplate { name: "Quad shooter", members: &[("simple_ufo_shooter", 4)] }, // 24
    WaveTemplate { name: "Harcèlement", members: &[("kamikaze", 3), ("green_ufo", 2), ("asteroid", 2)] }, // 25
    WaveTemplate { name: "Mines + ufos", members: &[("mine", 4), ("simple_ufo_wave", 1), ("kamikaze", 1)] }, // 25
    WaveTemplate { name: "Mixte lourd", members: &[("kamikaze", 3), ("green_ufo", 2), ("simple_ufo_shooter", 1)] }, // 29
    WaveTemplate { name: "Octopus + escorte", members: &[("octopus", 1), ("kamikaze", 3)] }, // 30

    // ─── Lourd (31-40) ───────────────────────────────────
    WaveTemplate { name: "Octopus + horde", members: &[("octopus", 1), ("simple_ufo_wave", 2)] }, // 31
    WaveTemplate { name: "Hordes massives", members: &[("simple_ufo_wave", 4)] }, // 32
    WaveTemplate { name: "Duo d'octopus", members: &[("octopus", 1), ("octopus_green", 1)] }, // 33
    WaveTemplate { name: "Massive mix", members: &[("simple_ufo_shooter", 2), ("kamikaze", 3), ("green_ufo", 3)] }, // 39

    // ─── Très lourd (40+) ────────────────────────────────
    WaveTemplate { name: "Salve finale", members: &[("simple_ufo_shooter", 3), ("kamikaze", 4), ("green_ufo", 2)] }, // 46
    WaveTemplate { name: "Apocalypse", members: &[("octopus_green", 1), ("simple_ufo_wave", 3), ("kamikaze", 3)] }, // 57
    WaveTemplate { name: "Triple octopus", members: &[("octopus", 2), ("octopus_green", 1)] }, // 48
];

/// Poids total d'un template = somme des `count × cost` de ses membres
/// d'après le pool d'ennemis. C'est cette métrique qui pilote la sélection
/// par tier de difficulté.
fn template_weight(template: &WaveTemplate, tunings: &[EnemyTuning]) -> u32 {
    template
        .members
        .iter()
        .map(|(name, count)| {
            let unit_cost = tunings
                .iter()
                .find(|t| t.name == *name)
                .map(|t| t.cost)
                .unwrap_or(0);
            count * unit_cost
        })
        .sum()
}

/// Sélectionne aléatoirement un template parmi ceux dont le poids tombe dans
/// la fenêtre `[target - WINDOW/2, target + WINDOW/2]` (target = `wave_weight_target(n)`).
/// Si aucun template ne match (cas extrême, ex: poids cible énorme), fallback
/// sur les templates les plus lourds disponibles.
fn template_for_wave(n: u32, tunings: &[EnemyTuning]) -> &'static WaveTemplate {
    let target = wave_weight_target(n);
    let half_window = WEIGHT_WINDOW / 2;
    let min_weight = target.saturating_sub(half_window);
    let max_weight = target + half_window;

    let candidates: Vec<&WaveTemplate> = WAVE_TEMPLATES
        .iter()
        .filter(|t| {
            let w = template_weight(t, tunings);
            w >= min_weight && w <= max_weight
        })
        .collect();

    if !candidates.is_empty() {
        return candidates[fastrand::usize(..candidates.len())];
    }

    // Fallback : si la fenêtre ne match rien (typiquement quand `target`
    // dépasse le template le plus lourd), prendre parmi le top 30% des
    // templates par poids pour garder du challenge.
    let mut sorted: Vec<(&WaveTemplate, u32)> = WAVE_TEMPLATES
        .iter()
        .map(|t| (t, template_weight(t, tunings)))
        .collect();
    sorted.sort_by_key(|(_, w)| *w);
    let cutoff = (sorted.len() * 7) / 10;
    let top: Vec<&WaveTemplate> = sorted[cutoff..].iter().map(|(t, _)| *t).collect();
    top[fastrand::usize(..top.len())]
}

// Note : on n'utilise PAS de marker `WaveMember`. Marker via `Added<Enemy>`
// avait un timing bug d'1 frame entre le spawn et l'attribution du marker,
// pouvant faire chuter le compteur à 0 prématurément. À la place, on compte
// directement `Enemy` + `SimpleUfoWaveSpawner` — en mode Vagues, ces
// entités N'EXISTENT que pendant les vagues, donc le count est fiable.

#[derive(Clone)]
pub enum WavesState {
    /// Settle entre deux vagues — courte respiration après le dernier kill.
    Idle { settle_timer: Timer },
    /// Vague en cours de spawn (déversement progressif des membres).
    Spawning {
        template_name: &'static str,
        queue: Vec<(&'static str, u32)>,
        spawn_timer: Timer,
    },
    /// Tous les membres ont été spawn — on attend leur mort.
    Waiting,
}

#[derive(Resource, Clone)]
pub struct WavesConfig {
    pub enemies: Vec<EnemyTuning>,
    pub spawn_position: SpawnPosition,
    pub wave_number: u32,
    pub state: WavesState,
}

impl Default for WavesConfig {
    fn default() -> Self {
        Self {
            enemies: default_tunings(),
            spawn_position: SpawnPosition::Top,
            wave_number: 0,
            // Pause initiale très courte : la 1re vague démarre vite.
            state: WavesState::Idle {
                settle_timer: Timer::from_seconds(0.5, TimerMode::Once),
            },
        }
    }
}

/// Système principal : tick la machine à état des vagues.
pub fn waves_spawner_system(
    time: Res<Time>,
    mut waves: ResMut<WavesConfig>,
    mut difficulty: ResMut<Difficulty>,
    mut commands: Commands,
    enemies_q: Query<(), With<Enemy>>,
    spawners_q: Query<(), With<SimpleUfoWaveSpawner>>,
) {
    let dt = time.delta();
    let waves = &mut *waves;
    let enemies_snapshot = waves.enemies.clone();
    let spawn_position = waves.spawn_position;

    let next_state: Option<WavesState> = match &mut waves.state {
        WavesState::Idle { settle_timer } => {
            settle_timer.tick(dt);
            if !settle_timer.is_finished() {
                None
            } else {
                let next_wave = waves.wave_number + 1;
                let template = template_for_wave(next_wave, &enemies_snapshot);
                let queue: Vec<(&'static str, u32)> = template.members.to_vec();
                let mut spawn_timer =
                    Timer::from_seconds(SPAWN_INTERVAL, TimerMode::Repeating);
                // Précharge pour que le 1er spawn parte immédiatement.
                let dur = spawn_timer.duration();
                spawn_timer.set_elapsed(dur);
                Some(WavesState::Spawning {
                    template_name: template.name,
                    queue,
                    spawn_timer,
                })
            }
        }
        WavesState::Spawning { queue, spawn_timer, .. } => {
            spawn_timer.tick(dt);
            if spawn_timer.just_finished() && !queue.is_empty() {
                // Pop le 1er membre, spawn 1 instance, décrémente.
                let (name, ref mut count) = queue[0];
                if let Some(tuning) = enemies_snapshot.iter().find(|t| t.name == name) {
                    spawn_picked(&mut commands, &mut difficulty, tuning, spawn_position);
                }
                *count -= 1;
                if *count == 0 {
                    queue.remove(0);
                }
            }
            if queue.is_empty() {
                Some(WavesState::Waiting)
            } else {
                None
            }
        }
        WavesState::Waiting => {
            // Vague terminée quand AUCUN ennemi ni spawner Bezier n'existe.
            // Compter le spawner séparément couvre le cas "le SimpleUfoWaveSpawner
            // est en cours mais aucun ufo n'est encore dans le world".
            let alive = enemies_q.iter().count() + spawners_q.iter().count();
            if alive == 0 {
                Some(WavesState::Idle {
                    settle_timer: Timer::from_seconds(SETTLE_DURATION, TimerMode::Once),
                })
            } else {
                None
            }
        }
    };

    if let Some(next) = next_state {
        if matches!(next, WavesState::Spawning { .. }) {
            waves.wave_number += 1;
        }
        waves.state = next;
    }
}

// ─── UI compteur de vague ───────────────────────────────────────────

#[derive(Component)]
#[require(crate::GameplayEntity)]
pub struct WavesLevelUI;

pub fn setup_waves_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    existing: Query<Entity, With<WavesLevelUI>>,
) {
    if existing.iter().next().is_some() {
        return;
    }
    let font = asset_server.load("fonts/PressStart2P-Regular.ttf");
    commands.spawn((
        Text::new("VAGUE 0"),
        TextFont { font, font_size: 24.0, ..default() },
        TextColor(Color::srgba(0.3, 0.85, 1.0, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(20.0),
            left: Val::Percent(50.0),
            margin: UiRect::left(Val::Px(-100.0)),
            ..default()
        },
        WavesLevelUI,
    ));
}

pub fn update_waves_ui(
    waves: Res<WavesConfig>,
    enemies_q: Query<(), With<Enemy>>,
    spawners_q: Query<(), With<SimpleUfoWaveSpawner>>,
    mut text_q: Query<&mut Text, With<WavesLevelUI>>,
) {
    let Ok(mut text) = text_q.single_mut() else { return };
    let status = match &waves.state {
        WavesState::Idle { settle_timer } => {
            let remaining =
                (settle_timer.duration().as_secs_f32() - settle_timer.elapsed_secs()).max(0.0);
            format!("VAGUE {} dans {:.1}s", waves.wave_number + 1, remaining)
        }
        WavesState::Spawning { template_name, .. } => {
            format!("VAGUE {} — {}", waves.wave_number, template_name)
        }
        WavesState::Waiting => {
            let alive = enemies_q.iter().count() + spawners_q.iter().count();
            format!("VAGUE {} — {} restants", waves.wave_number, alive)
        }
    };
    **text = status;
}
