//! Countdown UI : affiche READY → 3 → 2 → 1 → GO! au centre de l'écran
//! avec les sons correspondants et des animations dynamiques style jeu de course.
//!
//! Chaque étape pop avec un effet de scale (zoom-in + overshoot) puis fade-out.
//! Envoyez un `CountdownEvent` pour déclencher un countdown de 3 secondes.

use crate::difficulty::BoomEvent;
use crate::state::GameState;
use bevy::prelude::*;

pub struct CountdownPlugin;

impl Plugin for CountdownPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<CountdownEvent>()
            .add_systems(
                Update,
                (start_countdown, update_countdown, animate_countdown_text)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(OnExit(GameState::Playing), cleanup_countdown);
    }
}

/// Événement pour déclencher un countdown.
#[derive(Event)]
pub struct CountdownEvent;

/// Durée totale du countdown (secondes).
const COUNTDOWN_DURATION: f32 = 3.0;

/// Étapes du countdown : (temps relatif, texte, son).
const STEPS: &[(f32, &str, &str)] = &[
    (0.0, "READY", "audio/t_ready.ogg"),
    (0.75, "3", "audio/t_1.ogg"),
    (1.5, "2", "audio/t_1.ogg"),
    (2.25, "1", "audio/t_1.ogg"),
    (3.0, "GO!", "audio/t_go.wav"),
];

/// Durée d'affichage de "GO!" avant de disparaître.
const GO_LINGER: f32 = 0.5;

/// Durée de l'animation de pop pour chaque étape (secondes).
const POP_DURATION: f32 = 0.35;

/// Scale max au pic de l'overshoot.
const POP_OVERSHOOT: f32 = 1.4;

#[derive(Component)]
struct CountdownUI;

/// Animation de pop sur le texte du countdown.
#[derive(Component)]
struct CountdownPop {
    timer: f32,
    duration: f32,
}

#[derive(Resource)]
struct CountdownState {
    timer: f32,
    current_step: usize,
    finished: bool,
}

fn start_countdown(
    mut commands: Commands,
    mut events: EventReader<CountdownEvent>,
    asset_server: Res<AssetServer>,
    existing_q: Query<Entity, With<CountdownUI>>,
) {
    if events.read().next().is_none() {
        return;
    }
    events.read().for_each(drop);

    // Nettoyer un countdown précédent
    for entity in existing_q.iter() {
        commands.entity(entity).despawn_recursive();
    }

    let font = asset_server.load("fonts/PressStart2P-Regular.ttf");

    // Container centré plein écran
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                ..default()
            },
            CountdownUI,
        ))
        .with_children(|parent| {
            parent.spawn((
                TextBundle {
                    text: Text::from_section(
                        "READY",
                        TextStyle {
                            font,
                            font_size: 80.0,
                            color: Color::WHITE,
                        },
                    ),
                    style: Style { ..default() },
                    transform: Transform::from_scale(Vec3::splat(0.0)),
                    ..default()
                },
                CountdownPop {
                    timer: 0.0,
                    duration: POP_DURATION,
                },
            ));
        });

    // Son READY
    commands.spawn(AudioBundle {
        source: asset_server.load(STEPS[0].2),
        settings: PlaybackSettings::DESPAWN,
    });

    commands.insert_resource(CountdownState {
        timer: 0.0,
        current_step: 0,
        finished: false,
    });
}

fn update_countdown(
    mut commands: Commands,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    mut state: Option<ResMut<CountdownState>>,
    mut text_q: Query<(&mut Text, &mut CountdownPop), With<Parent>>,
    ui_q: Query<Entity, With<CountdownUI>>,
    mut boom_events: EventWriter<BoomEvent>,
) {
    let Some(ref mut state) = state else {
        return;
    };

    if state.finished {
        state.timer += time.delta_seconds();
        if state.timer >= COUNTDOWN_DURATION + GO_LINGER {
            for entity in ui_q.iter() {
                commands.entity(entity).despawn_recursive();
            }
            commands.remove_resource::<CountdownState>();
        }
        return;
    }

    state.timer += time.delta_seconds();

    let next_step = state.current_step + 1;
    if next_step < STEPS.len() && state.timer >= STEPS[next_step].0 {
        state.current_step = next_step;
        let (_, label, sound) = STEPS[next_step];

        for (mut text, mut pop) in text_q.iter_mut() {
            text.sections[0].value = label.to_string();

            if label == "GO!" {
                text.sections[0].style.color = Color::rgba(1.0, 0.85, 0.0, 1.0);
                text.sections[0].style.font_size = 120.0;
            } else {
                text.sections[0].style.color = Color::WHITE;
                text.sections[0].style.font_size = 100.0;
            }

            // Reset l'animation de pop
            pop.timer = 0.0;
        }

        commands.spawn(AudioBundle {
            source: asset_server.load(sound),
            settings: PlaybackSettings::DESPAWN,
        });

        if label == "GO!" {
            boom_events.send(BoomEvent);
            state.finished = true;
            state.timer = COUNTDOWN_DURATION;
        }
    }
}

/// Anime le texte du countdown : zoom-in avec overshoot puis stabilisation + léger fade-out en fin.
fn animate_countdown_text(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut Text, &mut CountdownPop)>,
) {
    for (mut transform, mut text, mut pop) in query.iter_mut() {
        pop.timer += time.delta_seconds();
        let t = (pop.timer / pop.duration).clamp(0.0, 1.0);

        // Courbe d'animation : overshoot élastique
        // Phase 1 (0→0.5) : scale 0 → POP_OVERSHOOT (ease-out)
        // Phase 2 (0.5→1.0) : scale POP_OVERSHOOT → 1.0 (ease-in-out)
        let scale = if t < 0.5 {
            let t2 = t / 0.5;
            let ease = 1.0 - (1.0 - t2).powi(3); // ease-out cubic
            ease * POP_OVERSHOOT
        } else {
            let t2 = (t - 0.5) / 0.5;
            let ease = t2 * t2 * (3.0 - 2.0 * t2); // smoothstep
            POP_OVERSHOOT + (1.0 - POP_OVERSHOOT) * ease
        };

        transform.scale = Vec3::splat(scale);

        // Fade-out léger après la fin de l'animation de pop (entre les étapes)
        let alpha = if pop.timer > pop.duration + 0.2 {
            let fade_t = ((pop.timer - pop.duration - 0.2) / 0.15).clamp(0.0, 1.0);
            1.0 - fade_t * 0.3 // fade partiel, pas complètement invisible
        } else {
            1.0
        };

        let base_color = text.sections[0].style.color;
        let r = base_color.r();
        let g = base_color.g();
        let b = base_color.b();
        text.sections[0].style.color = Color::rgba(r, g, b, alpha);
    }
}

fn cleanup_countdown(mut commands: Commands, query: Query<Entity, With<CountdownUI>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
    commands.remove_resource::<CountdownState>();
}
