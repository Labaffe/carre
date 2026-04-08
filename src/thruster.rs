use crate::difficulty::Difficulty;
use crate::player::Player;
use crate::state::GameState;
use bevy::prelude::*;
use bevy::sprite::Anchor;

pub struct ThrusterPlugin;

impl Plugin for ThrusterPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ThrusterSounds::default())
            .add_systems(OnEnter(GameState::Playing), reset_thruster_sounds)
            .add_systems(
                Update,
                (attach_thruster, animate_thruster).run_if(in_state(GameState::Playing)),
            );
    }
}

#[derive(Resource, Default)]
struct ThrusterSounds {
    charging_played: bool,
    boom_played: bool,
}

#[derive(Component)]
pub struct Thruster;

/// Alpha maximum de cette tranche (gradient baked à la création).
#[derive(Component)]
struct ThrusterLayer {
    max_alpha: f32,
}

// Nombre de tranches par rectangle — plus c'est élevé, plus le dégradé est lisse.
const SLICES: u32 = 12;

/// Définition d'un rectangle dégradé.
struct RectDef {
    width: f32,
    height: f32,
    color_rgb: [f32; 3], // RGB sans alpha
    top_alpha: f32,      // opacité en haut
    z: f32,
}

/// Spawn N tranches horizontales formant un dégradé du haut (opaque) vers le bas (transparent).
/// Retourne les entités créées (à attacher comme enfants du joueur).
fn spawn_gradient_rect(commands: &mut Commands, def: &RectDef, top_y: f32) -> Vec<Entity> {
    let slice_h = def.height / SLICES as f32;
    let [r, g, b] = def.color_rgb;
    let mut ids = Vec::new();

    for i in 0..SLICES {
        let t = i as f32 / (SLICES - 1) as f32; // 0 = haut, 1 = bas
        let max_alpha = def.top_alpha * (1.0 - t);
        // largeur décroissante : pleine en haut, fine en bas (effet flamme)
        let width = def.width * (1.0 - t * 0.88).max(0.05);
        let y = top_y - i as f32 * slice_h;

        let id = commands
            .spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: Color::rgba(r, g, b, 0.0),
                        custom_size: Some(Vec2::new(width, slice_h + 0.5)),
                        anchor: Anchor::TopCenter,
                        ..default()
                    },
                    transform: Transform::from_xyz(0.0, y, def.z),
                    ..default()
                },
                Thruster,
                ThrusterLayer { max_alpha },
            ))
            .id();

        ids.push(id);
    }

    ids
}

fn attach_thruster(mut commands: Commands, new_players: Query<Entity, Added<Player>>) {
    for player_entity in new_players.iter() {
        // Rectangle extérieur : orange, large
        let outer = spawn_gradient_rect(
            &mut commands,
            &RectDef {
                width: 28.0,
                height: 48.0,
                color_rgb: [1.0, 0.35, 0.0],
                top_alpha: 0.55,
                z: -0.2,
            },
            -32.0,
        );

        // Rectangle intérieur : jaune-blanc, étroit
        let inner = spawn_gradient_rect(
            &mut commands,
            &RectDef {
                width: 10.0,
                height: 36.0,
                color_rgb: [1.0, 0.85, 0.3],
                top_alpha: 0.95,
                z: -0.1,
            },
            -32.0,
        );

        let all: Vec<Entity> = outer.into_iter().chain(inner).collect();
        commands.entity(player_entity).push_children(&all);
    }
}

fn reset_thruster_sounds(mut sounds: ResMut<ThrusterSounds>) {
    *sounds = ThrusterSounds::default();
}

fn animate_thruster(
    difficulty: Res<Difficulty>,
    time: Res<Time>,
    mut thruster_q: Query<(&mut Sprite, &ThrusterLayer), With<Thruster>>,
    mut sounds: ResMut<ThrusterSounds>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let elapsed = difficulty.elapsed;
    let t = time.elapsed_seconds();

    // sons déclenchés une seule fois aux bons moments
    if elapsed >= 7.0 && !sounds.charging_played {
        sounds.charging_played = true;
        commands.spawn(AudioBundle {
            source: asset_server.load("audio/charging.ogg"),
            settings: PlaybackSettings::ONCE,
        });
    }
    if elapsed >= 10.0 && !sounds.boom_played {
        sounds.boom_played = true;
        commands.spawn(AudioBundle {
            source: asset_server.load("audio/boom.wav"),
            settings: PlaybackSettings::ONCE,
        });
    }

    let factor: f32 = if elapsed < 7.0 {
        // éteint
        0.0
    } else if elapsed < 10.0 {
        // mise en route : scintillement violent + enveloppe montante
        let ramp = (elapsed - 7.0) / 3.0; // 0→1 sur 3s
        let flicker = (t * 11.0).sin() * 0.70   // basse fréquence : coupures brutales
            + (t * 23.0).sin() * 0.50   // fréquence moyenne
            + (t * 61.0).sin() * 0.35   // haute fréquence
            + (t * 97.0).sin() * 0.20; // très haute : grain
        let flicker_norm = (flicker * 0.45 + 0.55).clamp(0.0, 1.0);
        // début très saccadé, fin proche de la pleine puissance
        (ramp * ramp * 0.3 + flicker_norm * ramp * 1.1).clamp(0.0, 1.0)
    } else {
        // pleine puissance : crépitement visible mais pas dominant
        let crackle = (t * 31.0).sin() * 0.10 + (t * 59.0).sin() * 0.07 + (t * 97.0).sin() * 0.05;
        (0.85 + crackle).clamp(0.0, 1.0)
    };

    for (mut sprite, layer) in thruster_q.iter_mut() {
        sprite.color.set_a(layer.max_alpha * factor);
    }
}
