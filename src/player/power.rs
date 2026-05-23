//! Abstraction des pouvoirs de la barre Espace.
//!
//! ## Architecture
//!
//! - [`PowerKind`] : enum centralisant tous les pouvoirs disponibles +
//!   métadonnées (nom, durée de cooldown, couleurs UI).
//! - [`EquippedPower`] : composant `EquippedPower(PowerKind)` porté par le
//!   joueur, indique quel pouvoir est équipé.
//! - [`PowerCooldowns`] : Resource avec un timer par `PowerKind`, tick
//!   centralement chaque frame. Les pouvoirs lisent via `is_ready` et
//!   reset via `trigger`.
//! - **UI unifiée** : un seul panneau bas-écran qui affiche le nom + la
//!   jauge du pouvoir équipé. Dispatch via `PowerKind::name` / colors.
//!
//! ## Ajouter un pouvoir
//!
//! 1. Ajouter une variante à `PowerKind` + ses méthodes (`name`,
//!    `cooldown_seconds`, `ready_color`, `charging_color`).
//! 2. L'ajouter à `PowerKind::ALL` pour qu'il ait un timer cooldown init.
//! 3. Créer son système d'input qui check :
//!    ```ignore
//!    if equipped.0 != PowerKind::MonPouvoir { return; }
//!    if !cooldowns.is_ready(PowerKind::MonPouvoir) { return; }
//!    ```
//! 4. À l'activation : `cooldowns.trigger(PowerKind::MonPouvoir);` +
//!    insérer composants/visuels propres au pouvoir.
//!
//! ## Swap (côté deckbuilding)
//!
//! ```ignore
//! // Query<&mut EquippedPower, With<Player>>
//! equipped.0 = PowerKind::Shield;
//! ```
//!
//! Atomique. Le système d'input de l'ancien pouvoir cesse de répondre à
//! Espace au tick suivant. L'UI se met à jour automatiquement.

use std::time::Duration;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::game_manager::state::GameState;
use crate::menu::pause::not_paused;
use crate::player::player::Player;

// ─── Enum + métadonnées ─────────────────────────────────────────────

#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PowerKind {
    Dash,
    Shield,
}

impl PowerKind {
    /// Liste exhaustive des pouvoirs (pour init des cooldowns + énumération
    /// éventuelle côté deckbuilding/menu). À mettre à jour quand un nouveau
    /// pouvoir est ajouté.
    pub const ALL: &'static [PowerKind] = &[Self::Dash, Self::Shield];

    pub fn name(&self) -> &'static str {
        match self {
            Self::Dash => "DASH",
            Self::Shield => "BOUCLIER",
        }
    }

    /// Durée du cooldown du pouvoir (secondes). Lue à l'init de
    /// `PowerCooldowns` pour configurer le timer correspondant.
    pub fn cooldown_seconds(&self) -> f32 {
        match self {
            Self::Dash => 5.0,
            Self::Shield => 8.0,
        }
    }

    /// Couleur du label + barre quand le pouvoir est prêt (cooldown fini).
    pub fn ready_color(&self) -> Color {
        match self {
            Self::Dash => Color::srgba(1.0, 0.85, 0.0, 1.0), // jaune vif
            Self::Shield => Color::srgba(0.25, 0.8, 1.0, 1.0), // cyan vif
        }
    }

    /// Couleur de la barre pendant la charge (cooldown en cours).
    pub fn charging_color(&self) -> Color {
        match self {
            Self::Dash => Color::srgba(0.3, 0.7, 1.0, 1.0), // cyan moyen
            Self::Shield => Color::srgba(0.2, 0.4, 0.7, 1.0), // bleu sombre
        }
    }
}

/// Composant porté par le joueur : pouvoir équipé sur la barre Espace.
#[derive(Component, Clone, Copy)]
pub struct EquippedPower(pub PowerKind);

// ─── Resource cooldowns ─────────────────────────────────────────────

/// Resource centralisant le cooldown de chaque `PowerKind`. Un timer par
/// pouvoir, tick chaque frame via `tick_cooldowns`. Init à "tous prêts".
#[derive(Resource)]
pub struct PowerCooldowns {
    timers: HashMap<PowerKind, Timer>,
}

impl Default for PowerCooldowns {
    fn default() -> Self {
        let mut timers = HashMap::new();
        for kind in PowerKind::ALL {
            let dur = kind.cooldown_seconds();
            let mut t = Timer::from_seconds(dur, TimerMode::Once);
            // État initial = prêt : tick à la durée complète.
            t.tick(Duration::from_secs_f32(dur));
            timers.insert(*kind, t);
        }
        Self { timers }
    }
}

impl PowerCooldowns {
    pub fn is_ready(&self, kind: PowerKind) -> bool {
        self.timers.get(&kind).map_or(false, |t| t.is_finished())
    }

    /// Reset le timer (= démarre le cooldown). Appelé après activation.
    pub fn trigger(&mut self, kind: PowerKind) {
        if let Some(t) = self.timers.get_mut(&kind) {
            t.reset();
        }
    }

    /// Progression du cooldown : 0 = vient d'être déclenché, 1 = prêt.
    pub fn fraction(&self, kind: PowerKind) -> f32 {
        self.timers.get(&kind).map_or(1.0, |t| t.fraction())
    }

    pub fn tick_all(&mut self, delta: Duration) {
        for t in self.timers.values_mut() {
            t.tick(delta);
        }
    }
}

// ─── Plugin ─────────────────────────────────────────────────────────

pub struct PowerPlugin;

impl Plugin for PowerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PowerCooldowns>()
            .add_systems(
                OnEnter(GameState::Playing),
                (reset_cooldowns, setup_power_ui),
            )
            // Cleanup UI : géré centralement par `cleanup_playing` (main.rs)
            // via `#[require(GameplayEntity)]` sur `PowerUIRoot`.
            .add_systems(
                Update,
                (tick_cooldowns, update_power_ui)
                    .run_if(in_state(GameState::Playing))
                    .run_if(not_paused),
            );
    }
}

fn reset_cooldowns(mut cooldowns: ResMut<PowerCooldowns>) {
    *cooldowns = PowerCooldowns::default();
}

fn tick_cooldowns(time: Res<Time>, mut cooldowns: ResMut<PowerCooldowns>) {
    cooldowns.tick_all(time.delta());
}

// ─── UI unifiée ─────────────────────────────────────────────────────

const POWER_UI_BAR_WIDTH: f32 = 120.0;
const POWER_UI_BAR_HEIGHT: f32 = 8.0;

#[derive(Component)]
#[require(crate::GameplayEntity)]
struct PowerUIRoot;
#[derive(Component)]
struct PowerUILabel;
#[derive(Component)]
struct PowerUIBar;

fn setup_power_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/PressStart2P-Regular.ttf");
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(20.0),
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(-POWER_UI_BAR_WIDTH / 2.0)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(4.0),
                ..default()
            },
            PowerUIRoot,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(""),
                TextFont {
                    font,
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                PowerUILabel,
            ));
            parent
                .spawn((
                    Node {
                        width: Val::Px(POWER_UI_BAR_WIDTH),
                        height: Val::Px(POWER_UI_BAR_HEIGHT),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.15, 0.15, 0.15, 0.8)),
                ))
                .with_children(|track| {
                    track.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(Color::WHITE),
                        PowerUIBar,
                    ));
                });
        });
}

/// Lit `EquippedPower` du joueur et le `PowerCooldowns`, met à jour le
/// label (nom + couleur) et la barre (largeur + couleur) selon le pouvoir
/// équipé. Un changement d'EquippedPower est répercuté à la frame suivante.
fn update_power_ui(
    cooldowns: Res<PowerCooldowns>,
    equipped: Single<&EquippedPower, With<Player>>,
    mut label_q: Query<(&mut Text, &mut TextColor), With<PowerUILabel>>,
    mut bar_q: Query<(&mut Node, &mut BackgroundColor), With<PowerUIBar>>,
) {
    let kind = equipped.0;
    let ready = cooldowns.is_ready(kind);
    let fraction = cooldowns.fraction(kind);

    if let Ok((mut text, mut color)) = label_q.single_mut() {
        let new_name = kind.name();
        if **text != new_name {
            **text = new_name.to_string();
        }
        color.0 = if ready {
            kind.ready_color()
        } else {
            Color::srgba(0.45, 0.45, 0.45, 1.0)
        };
    }
    if let Ok((mut node, mut bg)) = bar_q.single_mut() {
        node.width = Val::Percent(fraction * 100.0);
        bg.0 = if ready {
            kind.ready_color()
        } else {
            kind.charging_color()
        };
    }
}

// cleanup_power_ui retiré — cleanup auto via `cleanup_playing` (main.rs) grâce
// à `#[require(GameplayEntity)]` sur `PowerUIRoot`.
