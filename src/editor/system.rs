use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::app::AppExit;
use bevy::scene::ron::value::Float;
use bevy::utils::Duration;
use bevy::time::Stopwatch;
use std::{collections::btree_map::Range, f32::consts::*};
use crate::{behavior::{behavior::*, indexed_node_list::*,component_container::*, ordered_list::*, parallel_node_list::ParallelNodeList}, movement::goto::Goto};
use crate::movement::{movements::Movements,sinusoid::Sinusoid};
#[derive(Component)]
struct some_enemy_main_component {}

fn spawn(
    shift:f32,
    mut commands: Commands, 
    frames:Handle<Image>
) {
    let goto = Goto::new(Vec2::ZERO,10.0);
    let cardio:Movements = Movements::new()
        .with(Sinusoid::new(100.0,1.0,0.0,Vec2::new(1.0,0.0)))
        .with(Sinusoid::new(100.0,2.0,0.0,Vec2::new(0.0,1.0)))
        .with(goto.clone())
        ;

    let circle:Movements = Movements::new()
        .with(Sinusoid::new(100.0,1.0,0.0,Vec2::new(1.0,0.0)))
        .with(Sinusoid::new(100.0,1.0,PI * 0.5+0.0,Vec2::new(0.0,1.0)))
        .with(goto)
        ;

    let behavior =  OrderedNodeList::new()
        .then(Duration::from_secs(5),ComponentContainer::new( cardio))
        .then(Duration::from_secs(5),ComponentContainer::new( circle))
        .should_loop();
    commands.spawn((
        SpriteBundle {
            texture: frames,
            sprite: Sprite {
                custom_size: Some(Vec2::splat(50.0)),
                ..default()
            },
            transform: Transform::from_xyz(shift, 0.0, 0.5),
            ..default()
        },
        BehaviorComponent::new(behavior)
    ));
}

pub fn init(
    mut commands: Commands, 
    asset_server: Res<AssetServer>
) {
    let frames = asset_server.load("images/asteroids/asteroid_x002.png");
    for n in 0..50 {
        spawn((n as f32) * 0.2,commands.reborrow(),frames.clone());
    }
}

pub fn pause(mut exit: EventWriter<AppExit>, keyboard: Res<ButtonInput<KeyCode>>,) {
    if keyboard.just_pressed(KeyCode::Escape) {
        exit.send(AppExit::Success);
    }
}