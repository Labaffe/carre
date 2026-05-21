//! Countdown UI : affiche READY → 3 → 2 → 1 → GO! au centre de l'écran
//! avec les sons correspondants et des animations dynamiques style jeu de course.
//!
//! Chaque étape pop avec un effet de scale (zoom-in + overshoot) puis fade-out.
//! Envoyez un `CountdownEvent` pour déclencher un countdown de 3 secondes.

use crate::audio::{Sfx, SfxPlayer};
use crate::game_manager::difficulty::BoomEvent;
use crate::game_manager::state::GameState;
use bevy::prelude::*;

pub struct CountdownPlugin;

impl Plugin for CountdownPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<CountdownEvent>()
            .add_systems(OnEnter(GameState::Playing), warmup_countdown_fonts)
            .add_systems(
                Update,
                (
                    start_countdown,
                    update_countdown,
                    animate_countdown_text,
                    cleanup_warmup_fonts,
                )
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(OnExit(GameState::Playing), cleanup_countdown);
    }
}

/// Événement pour déclencher un countdown.
#[derive(Message)]
pub struct CountdownEvent;

/// Durée totale du countdown (secondes).
const COUNTDOWN_DURATION: f32 = 3.0;

/// Étapes du countdown : (temps relatif, texte, son).
const STEPS: &[(f32, &str, Sfx)] = &[
    (0.0,  "READY", Sfx::UiCountdownReady),
    (0.75, "3",     Sfx::UiCountdownBeep),
    (1.5,  "2",     Sfx::UiCountdownBeep),
    (2.25, "1",     Sfx::UiCountdownBeep),
    (3.0,  "GO!",   Sfx::UiCountdownGo),
];

/// Durée d'affichage de "GO!" avant de disparaître.
const GO_LINGER: f32 = 0.5;

/// Durée de l'animation de pop pour chaque étape (secondes).
const POP_DURATION: f32 = 0.35;

/// Scale max au pic de l'overshoot.
const POP_OVERSHOOT: f32 = 1.4;

/// Pas de quantification du `font_size` pendant le pop. Aligne les tailles
/// rendues avec celles du warmup (`warmup_countdown_fonts`), pour que tous
/// les rasterizations d'atlas de glyphes soient déjà en cache. Sans ça, le
/// pop génère ~21 tailles uniques en 0.35s = autant de re-rastérisations.
const FONT_SIZE_QUANTUM: f32 = 4.0;

/// Plus grande `font_size` attendue pendant le pop (max BaseFontSize 120
/// × POP_OVERSHOOT 1.4 ≈ 168, arrondi). Tous les multiples de
/// FONT_SIZE_QUANTUM de 4 à FONT_SIZE_MAX_WARMUP sont précompilés.
const FONT_SIZE_MAX_WARMUP: f32 = 172.0;

#[derive(Component)]
struct CountdownUI;

/// Marker sur le container off-screen qui contient les textes warmup.
/// Auto-despawn après que son timer expire (le temps que Bevy rastérise les
/// glyphes au 1er render de chaque taille).
#[derive(Component)]
struct CountdownFontWarmup {
    timer: Timer,
}

/// Animation de pop sur le texte du countdown.
#[derive(Component)]
struct CountdownPop {
    timer: f32,
    duration: f32,
}

/// Taille de police de référence pour le zoom dynamique du countdown.
#[derive(Component)]
struct BaseFontSize(f32);

#[derive(Resource)]
struct CountdownState {
    timer: f32,
    current_step: usize,
    finished: bool,
}

fn start_countdown(
    mut commands: Commands,
    mut events: MessageReader<CountdownEvent>,
    asset_server: Res<AssetServer>,
    existing_q: Query<Entity, With<CountdownUI>>,
    mut sfx: SfxPlayer,
) {
    if events.read().next().is_none() {
        return;
    }
    events.read().for_each(drop);

    // Nettoyer un countdown précédent
    for entity in existing_q.iter() {
        if let Ok(mut e) = commands.get_entity(entity) {
            e.try_despawn();
        }
    }

    let font = asset_server.load("fonts/PressStart2P-Regular.ttf");

    // Container centré plein écran
    commands
        .spawn((
            (
            Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
        ),
            CountdownUI,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("READY"),
                TextFont { font, font_size: 0.0, ..default() },
                TextColor(Color::WHITE),
                Node::default(),
                BaseFontSize(80.0),
                CountdownPop {
                    timer: 0.0,
                    duration: POP_DURATION,
                },
            ));
        });

    sfx.play(STEPS[0].2);

    commands.insert_resource(CountdownState {
        timer: 0.0,
        current_step: 0,
        finished: false,
    });
}

fn update_countdown(
    mut commands: Commands,
    time: Res<Time>,
    mut state: Option<ResMut<CountdownState>>,
    mut text_q: Query<(&mut Text, &mut TextColor, &mut BaseFontSize, &mut CountdownPop), With<ChildOf>>,
    ui_q: Query<Entity, With<CountdownUI>>,
    mut boom_events: MessageWriter<BoomEvent>,
    mut sfx: SfxPlayer,
) {
    let Some(ref mut state) = state else {
        return;
    };

    if state.finished {
        state.timer += time.delta_secs();
        if state.timer >= COUNTDOWN_DURATION + GO_LINGER {
            for entity in ui_q.iter() {
                if let Ok(mut e) = commands.get_entity(entity) {
                    e.try_despawn();
                }
            }
            commands.remove_resource::<CountdownState>();
        }
        return;
    }

    state.timer += time.delta_secs();

    let next_step = state.current_step + 1;
    if next_step < STEPS.len() && state.timer >= STEPS[next_step].0 {
        state.current_step = next_step;
        let (_, label, sound) = STEPS[next_step];

        for (mut text, mut text_color, mut base, mut pop) in text_q.iter_mut() {
            **text = label.to_string();

            if label == "GO!" {
                text_color.0 = Color::srgba(1.0, 0.85, 0.0, 1.0);
                base.0 = 120.0;
            } else {
                text_color.0 = Color::WHITE;
                base.0 = 100.0;
            }

            // Reset l'animation de pop
            pop.timer = 0.0;
        }

        sfx.play(sound);

        if label == "GO!" {
            boom_events.write(BoomEvent);
            state.finished = true;
            state.timer = COUNTDOWN_DURATION;
        }
    }
}

/// Anime le texte du countdown : zoom-in avec overshoot puis stabilisation + léger fade-out en fin.
/// Le zoom se fait via TextFont.font_size = BaseFontSize * scale, car Transform.scale
/// ne s'applique pas aux entités UI en Bevy 0.17+.
fn animate_countdown_text(
    time: Res<Time>,
    mut query: Query<(&mut TextFont, &BaseFontSize, &mut TextColor, &mut CountdownPop)>,
) {
    for (mut text_font, base, mut text_color, mut pop) in query.iter_mut() {
        pop.timer += time.delta_secs();
        let t = (pop.timer / pop.duration).clamp(0.0, 1.0);

        let scale = if t < 0.5 {
            let t2 = t / 0.5;
            let ease = 1.0 - (1.0 - t2).powi(3);
            ease * POP_OVERSHOOT
        } else {
            let t2 = (t - 0.5) / 0.5;
            let ease = t2 * t2 * (3.0 - 2.0 * t2);
            POP_OVERSHOOT + (1.0 - POP_OVERSHOOT) * ease
        };

        // Quantification : aligne sur le cache pré-rastérisé par le warmup.
        let raw_size = base.0 * scale;
        text_font.font_size = (raw_size / FONT_SIZE_QUANTUM).round() * FONT_SIZE_QUANTUM;

        let alpha = if pop.timer > pop.duration + 0.2 {
            let fade_t = ((pop.timer - pop.duration - 0.2) / 0.15).clamp(0.0, 1.0);
            1.0 - fade_t * 0.3
        } else {
            1.0
        };

        let base_srgba = text_color.0.to_srgba();
        text_color.0 = Color::srgba(
            base_srgba.red,
            base_srgba.green,
            base_srgba.blue,
            alpha,
        );
    }
}

/// À l'entrée du state Playing : spawn des textes "READY 3210 GO!" invisibles
/// (off-screen) à toutes les tailles quantifiées que le pop animation va
/// utiliser. Force Bevy à rastériser et cacher les atlas de glyphes au 1er
/// frame du niveau (lag invisible) plutôt qu'au moment du countdown (lag
/// visible). Auto-despawn après 0.5s (cache déjà chaud après 1-2 frames).
fn warmup_countdown_fonts(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/PressStart2P-Regular.ttf");
    // Tous les caractères qui apparaîtront pendant le countdown.
    let chars = "READY 3210 GO!";
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                // Très loin hors écran — le rendu se fait quand même
                // (Bevy ne cull pas via Node.position négative), donc les
                // glyphes sont rastérisés, mais pas visibles pour le joueur.
                top: Val::Px(-10000.0),
                left: Val::Px(-10000.0),
                ..default()
            },
            CountdownFontWarmup {
                timer: Timer::from_seconds(0.5, TimerMode::Once),
            },
        ))
        .with_children(|parent| {
            let mut size = FONT_SIZE_QUANTUM;
            while size <= FONT_SIZE_MAX_WARMUP {
                parent.spawn((
                    Text::new(chars),
                    TextFont {
                        font: font.clone(),
                        font_size: size,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
                size += FONT_SIZE_QUANTUM;
            }
        });
}

/// Tick le timer du warmup ; despawn dès qu'expiré. La courte durée (0.5s)
/// laisse à Bevy le temps de rendre les textes une fois (= rastériser tous
/// les atlas) avant cleanup.
fn cleanup_warmup_fonts(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut CountdownFontWarmup)>,
) {
    for (entity, mut warmup) in query.iter_mut() {
        warmup.timer.tick(time.delta());
        if warmup.timer.is_finished() {
            if let Ok(mut e) = commands.get_entity(entity) {
                e.try_despawn();
            }
        }
    }
}

fn cleanup_countdown(mut commands: Commands, query: Query<Entity, With<CountdownUI>>) {
    for entity in query.iter() {
        if let Ok(mut e) = commands.get_entity(entity) {
            e.try_despawn();
        }
    }
    commands.remove_resource::<CountdownState>();
}
