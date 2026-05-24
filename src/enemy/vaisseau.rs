//! Vaisseau — premier exemple de composition `EnemyGroup` :
//! un parent invisible + 3 tourelles enfants disposées horizontalement.
//!
//! Le parent porte le `Movements` (ici une oscillation horizontale), les
//! tourelles héritent du mouvement via la propagation `GlobalTransform`
//! Bevy. Le parent despawn dès que les 3 tourelles sont mortes via
//! `DespawnWhenChildrenEmpty`.
//!
//! Pas de sprite ni de collider sur le parent — il n'existe que pour porter
//! la position de la formation. Les seules entités attaquables sont les
//! tourelles enfants.

use std::time::Duration;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::enemy::enemy_builder::EnemyBuilder;
use crate::enemy::enemy_group::{DespawnWhenChildrenEmpty, EnemyGroup};
use crate::enemy::turret::{turret_bundle, turret_module_bundle};
use crate::game_manager::difficulty::{Difficulty, SpawnPosition};
use crate::movement::movements::Movements;
use crate::movement::oscilate::Oscilate;

/// Espacement horizontal entre tourelles dans la formation (px). 4 tourelles
/// sont disposées sur les multiples impairs : -1.5, -0.5, +0.5, +1.5 ×
/// spacing → largeur totale = 3 × spacing.
const TURRET_SPACING: f32 = 300.0;
/// Hauteur du vaisseau par rapport au centre de l'écran (px, +Y = haut).
/// Force le vaisseau à flotter dans la partie haute, indépendamment de la
/// `SpawnPosition` passée. Combiné avec l'amplitude Y faible, le vaisseau
/// reste cantonné au tiers supérieur.
const VAISSEAU_Y_ANCHOR: f32 = 220.0;
/// Amplitude horizontale du flottement (px, de part et d'autre de l'ancre).
const VAISSEAU_OSC_AMPLITUDE_X: f32 = 260.0;
/// Amplitude verticale du flottement — bien plus faible que l'horizontal
/// pour que le vaisseau "respire" sans descendre vers le joueur.
const VAISSEAU_OSC_AMPLITUDE_Y: f32 = 60.0;
/// Fréquence X de l'oscillateur (rad²/s²). Lent — flottement majestueux.
const VAISSEAU_OSC_FREQ_X: f32 = 0.4;
/// Fréquence Y, légèrement différente de X pour produire un motif de
/// Lissajous (au lieu d'un déplacement en ligne droite diagonale).
const VAISSEAU_OSC_FREQ_Y: f32 = 0.7;

pub struct VaisseauBuilder {
    timer: Timer,
}

impl VaisseauBuilder {
    pub fn new() -> Self {
        Self {
            timer: Timer::new(Duration::ZERO, TimerMode::Once),
        }
    }
}

impl EnemyBuilder for VaisseauBuilder {
    fn get_timer(&mut self) -> &mut Timer {
        &mut self.timer
    }
    fn name(&self) -> &'static str {
        "vaisseau"
    }
    fn preload_anim(&self) -> HashMap<&str, &str> {
        // Le vaisseau lui-même n'a pas d'animation propre. Les tourelles
        // utilisent l'anim `turret_fire` préchargée par `TurretBuilder`.
        HashMap::new()
    }
    fn spawn(
        &self,
        mut commands: Commands,
        _window: &Window,
        _difficulty: &ResMut<Difficulty>,
        _spawn_pos: SpawnPosition,
        asset_server: &Res<AssetServer>,
    ) {
        // Spawn fixe au centre-haut : la formation est large (3× TURRET_SPACING
        // = 900px) plus l'amplitude d'oscillation horizontale (±260px). Avec
        // un point d'ancrage random, les bords du vaisseau peuvent sortir
        // de l'écran sur les côtés. Forçant X=0, le vaisseau reste lisible
        // et tous les tourelles sont accessibles au joueur.
        let origin = Vec2::new(0.0, VAISSEAU_Y_ANCHOR);

        // Parent : marker group + cleanup + flottement (X et Y combinés) +
        // visibility (requis quand pas de Sprite, sinon les enfants ne sont
        // pas rendus).
        commands
            .spawn((
                EnemyGroup,
                DespawnWhenChildrenEmpty,
                Transform::from_xyz(origin.x, origin.y, 0.5),
                Visibility::default(),
                // Deux oscillateurs indépendants : X lent et large, Y
                // rapide et étroit → trajectoire Lissajous "flottante".
                Movements::new()
                    .with(Oscilate::new(
                        Vec2::X,
                        origin,
                        VAISSEAU_OSC_FREQ_X,
                        VAISSEAU_OSC_AMPLITUDE_X,
                    ))
                    .with(Oscilate::new(
                        Vec2::Y,
                        origin,
                        VAISSEAU_OSC_FREQ_Y,
                        VAISSEAU_OSC_AMPLITUDE_Y,
                    )),
            ))
            .with_children(|p| {
                // 4 slots horizontaux (offsets -1.5, -0.5, +0.5, +1.5 ×
                // spacing) : un module visuel + une tourelle Enemy par
                // slot, en entités séparées (siblings) pour que la mort
                // d'une tourelle ne casse pas son module. Position LOCALE
                // (relative au parent) — la propagation GlobalTransform
                // fait le reste.
                for mult in [-1.5_f32, -0.5, 0.5, 1.5] {
                    let offset_x = mult * TURRET_SPACING;
                    p.spawn(turret_module_bundle(
                        asset_server,
                        Vec3::new(offset_x, 0.0, -0.05),
                    ));
                    p.spawn(turret_bundle(asset_server, Vec3::new(offset_x, 0.0, 0.0)));
                }
            });
    }
}
