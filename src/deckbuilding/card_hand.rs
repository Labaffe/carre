use crate::state::GameState;
use bevy::prelude::*;
use crate::deckbuilding::cards::DummyCard;
use crate::deckbuilding::card_ui::{spawn_card_ui,CardUI};
pub struct CardHandPlugin;

impl Plugin for CardHandPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<HandVisible>()
            .add_systems(OnEnter(GameState::Playing), spawn_hand_ui)
            .add_systems(Update, (
                toggle_hand,
                animate_hand,
            ).run_if(in_state(GameState::Playing)));
    }
}


fn spawn_hand_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    for i in 0..5 {
        spawn_card_ui(commands.reborrow(), asset_server.clone(), DummyCard{}, i);
    }
}

#[derive(Resource, Default)]
pub struct HandVisible(pub bool);

fn toggle_hand(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut visible: ResMut<HandVisible>,
) {
    if keyboard.just_pressed(KeyCode::KeyU) {
        visible.0 = !visible.0;
    }
}
use crate::tweening::{Tween, TweenUIPos, Ease};
use crate::tweening;
fn animate_hand(
    mut commands: Commands,
    visible: Res<HandVisible>,
    windows: Query<&Window>,
    query: Query<(Entity, &CardUI, &Style)>,
) {
    if !visible.is_changed() {
        return;
    }
    let window = windows.single();
    let center_x = window.width() / 2.0;
    let y = window.height() * 0.5;

    let spacing = 120.0;

    for (entity, card_ui, style) in query.iter() {
        let i = card_ui.0 as f32;

        let target_x = if visible.0 {
            center_x + (i - 2.0) * spacing
        } else {
            -300.0 //- i * 200.0 // exit left
        };
        let from_x = if visible.0 {
            window.width()+300.0// - i * 200.0
        } else {
            center_x + (i - 2.0) * spacing // exit left
        };
        println!("{}",match style.top {Val::Px(f)=>"px".to_string(),Val::Percent(f)=>"percent".to_string(),_=>"other".to_string()});

        tweening::ui_pos(
            Vec2::new( from_x,y),
            Vec2::new(target_x, y),
            0.5,
            Ease::EaseOut
        ).play(&mut commands,entity);
    }
}