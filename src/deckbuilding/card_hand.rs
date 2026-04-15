use crate::state::GameState;
use bevy::prelude::*;
use crate::deckbuilding::cards::DummyCard;
use crate::deckbuilding::card_ui::{spawn_card_ui,CardUI};
pub struct CardHandPlugin;

impl Plugin for CardHandPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CardHandTime>().add_systems(OnEnter(GameState::Playing),spawn_hand_ui)
            .add_systems(Update, display_hand.run_if(in_state(GameState::Playing)));
            //.add_systems(Update, (score_update.run_if(in_state(GameState::Playing)),level_update.run_if(in_state(GameState::Playing))));
    }
}

#[derive(Resource)]
pub struct CardHandTime {
    value: f32
}

impl Default for CardHandTime {
    fn default() -> Self {
        CardHandTime {
            value: 0.0,
        }
    }
}
fn spawn_hand_ui(commands1:Commands,commands2:Commands,commands3:Commands,asset_server:Res<AssetServer>){
    spawn_card_ui(commands1,asset_server.clone(),DummyCard{},0);
    spawn_card_ui(commands2,asset_server.clone(),DummyCard{},1);
    spawn_card_ui(commands3,asset_server.clone(),DummyCard{},2);
}

fn display_hand(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut local_time: ResMut<CardHandTime>,
    mut card_uis: Query<(&CardUI, &mut Visibility, &mut Transform)>
) {
    local_time.value += time.delta_seconds();
    if keyboard.pressed(KeyCode::KeyU) {
        for (card_ui, mut vis, mut transform) in card_uis.iter_mut() {
            *vis = Visibility::Visible;
            transform.translation = Vec3::new(local_time.value.cos(),0.0,0.0);
        }
    }
}