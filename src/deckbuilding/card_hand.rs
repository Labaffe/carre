use crate::state::GameState;
use bevy::prelude::*;
use crate::deckbuilding::cards::DummyCard;
use crate::deckbuilding::card_ui::{spawn_card_ui,CardUI};
pub struct CardHandPlugin;

impl Plugin for CardHandPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CardHandState>()
            .add_systems(OnEnter(GameState::Playing), spawn_hand_ui)
            .add_systems(Update, (
                toggle_hand,
                animate_hand,
                layout_hand,
            ).run_if(in_state(GameState::Playing)));
    }
}

#[derive(Resource)]
pub struct CardHandTime {
    value: f32
}
#[derive(Resource)]
pub struct CardHandState {
    pub visible: bool,
    pub progress: f32, // 0.0 = hidden, 1.0 = fully shown
}

impl Default for CardHandState {
    fn default() -> Self {
        Self {
            visible: false,
            progress: 0.0,
        }
    }
}
impl Default for CardHandTime {
    fn default() -> Self {
        CardHandTime {
            value: 0.0,
        }
    }
}
fn spawn_hand_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    for i in 0..5 {
        spawn_card_ui(commands.reborrow(), asset_server.clone(), DummyCard{}, i);
    }
}
fn layout_hand(
    mut card_uis: Query<(&CardUI, &mut Style)>,
    state: Res<CardHandState>,
    windows: Query<&Window>,
) {
    let window = windows.single();
    let center_x = window.width() / 2.0;

    let travel_spacing = 300.0;
    let final_spacing = 120.0;

    let y = window.height() * 0.5;

    for (card_ui, mut style) in card_uis.iter_mut() {
        let i = card_ui.0 as f32;

        // --- FINAL POSITION (centered, tight)
        let final_x = center_x + (i - 2.0) * final_spacing;

        // --- START POSITION (off-screen right, wide)
        let start_x = window.width() + i * travel_spacing;

        // --- INTERPOLATION
        let t = state.progress;
        let x = start_x + (final_x - start_x) * t;

        style.position_type = PositionType::Absolute;
        style.left = Val::Px(x);
        style.top = Val::Px(y);

        // visibility
        style.display = if state.progress > 0.01 {
            Display::Flex
        } else {
            Display::None
        };
    }
}
fn toggle_hand(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<CardHandState>,
) {
    if keyboard.just_pressed(KeyCode::KeyU) {
        state.visible = !state.visible;
    }
}

fn animate_hand(
    time: Res<Time>,
    mut state: ResMut<CardHandState>,
) {
    let speed = 3.0;

    let target = if state.visible { 1.0 } else { 0.0 };

    state.progress += (target - state.progress) * speed * time.delta_seconds();

    // snap to avoid endless float drift
    if (state.progress - target).abs() < 0.01 {
        state.progress = target;
    }
}