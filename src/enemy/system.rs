use bevy::prelude::*;
use std::time::Duration;
// ============================================================================
// BEHAVIOR PRIMITIVES
// ============================================================================

/// A behavior is just a trait object that can execute each frame.
/// Behaviors are stateless and reusable across enemies.
pub trait Behavior: Send + Sync + 'static {
    fn execute(&self, entity: Entity, world: &mut World);
    fn name(&self) -> &'static str;
}

/// Wrapper for type-erased behaviors
#[derive(Clone)]
pub struct BehaviorBox(pub std::sync::Arc<dyn Behavior>);

impl BehaviorBox {
    pub fn new<B: Behavior>(behavior: B) -> Self {
        Self(std::sync::Arc::new(behavior))
    }
}

// ============================================================================
// BEHAVIOR COMBINATORS (the composable part)
// ============================================================================

/// Execute behaviors in sequence
pub struct Sequence(pub Vec<BehaviorBox>);

impl Behavior for Sequence {
    fn execute(&self, entity: Entity, world: &mut World) {
        for behavior in &self.0 {
            behavior.0.execute(entity, world);
        }
    }
    fn name(&self) -> &'static str { "Sequence" }
}

/// Pick one behavior at random
pub struct RandomChoice(pub Vec<BehaviorBox>);

impl Behavior for RandomChoice {
    fn execute(&self, entity: Entity, world: &mut World) {
        if self.0.is_empty() {
            return;
        }
        let idx = fastrand::usize(0..self.0.len());
        self.0[idx].0.execute(entity, world);
    }
    fn name(&self) -> &'static str { "RandomChoice" }
}

/// Weighted random selection
pub struct WeightedChoice(pub Vec<(f32, BehaviorBox)>);

impl Behavior for WeightedChoice {
    fn execute(&self, entity: Entity, world: &mut World) {
        let total: f32 = self.0.iter().map(|(w, _)| w.max(0.0)).sum();
        if total <= 0.0 {
            return;
        }
        let mut pick = fastrand::f32() * total;
        for (weight, behavior) in &self.0 {
            pick -= weight.max(0.0);
            if pick <= 0.0 {
                behavior.0.execute(entity, world);
                return;
            }
        }
    }
    fn name(&self) -> &'static str { "WeightedChoice" }
}

/// Conditional behavior
pub struct Conditional<F: Fn(Entity, &World) -> bool + Send + Sync + 'static> {
    pub condition: F,
    pub then_do: BehaviorBox,
    pub else_do: Option<BehaviorBox>,
}

impl<F: Fn(Entity, &World) -> bool + Send + Sync + 'static> Behavior for Conditional<F> {
    fn execute(&self, entity: Entity, world: &mut World) {
        if (self.condition)(entity, world) {
            self.then_do.0.execute(entity, world);
        } else if let Some(ref else_behavior) = self.else_do {
            else_behavior.0.execute(entity, world);
        }
    }
    fn name(&self) -> &'static str { "Conditional" }
}
// ============================================================================
// PHASES
// ============================================================================

/// Identifies a phase by name (could also use an enum per enemy type)
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PhaseId(pub &'static str);

/// A phase defines what behavior runs and when to transition out
#[derive(Clone)]
pub struct Phase {
    pub id: PhaseId,
    /// Optional one-shot behavior executed exactly once when this phase is entered.
    /// Use for actions like spawning minions, playing a sound, inserting a marker
    /// component that subsequent per-frame logic depends on.
    pub on_enter: Option<BehaviorBox>,
    /// Per-frame behavior while in this phase.
    pub behavior: BehaviorBox,
    pub transitions: Vec<Transition>,
}

impl Phase {
    /// Convenience builder : phase sans `on_enter`.
    pub fn new(id: PhaseId, behavior: BehaviorBox, transitions: Vec<Transition>) -> Self {
        Self {
            id,
            on_enter: None,
            behavior,
            transitions,
        }
    }
}

/// Transition triggers
#[derive(Clone)]
pub enum TransitionTrigger {
    /// After a duration
    Timer(Duration),
    /// When health falls below threshold (0.0 - 1.0)
    HealthBelow(f32),
    /// When health falls above threshold (for healing scenarios)
    HealthAbove(f32),
    /// External event by name
    Event(&'static str),
    /// Custom predicate
    Custom(std::sync::Arc<dyn Fn(Entity, &World) -> bool + Send + Sync>),
}

#[derive(Clone)]
pub struct Transition {
    pub trigger: TransitionTrigger,
    pub target_phase: PhaseId,
    /// Optional priority if multiple transitions could fire
    pub priority: i32,
}

// ============================================================================
// ENEMY DEFINITION (the declarative part)
// ============================================================================

/// An enemy archetype is fully defined by its phases
#[derive(Clone)]
pub struct EnemyDefinition {
    pub name: &'static str,
    pub initial_phase: PhaseId,
    pub phases: Vec<Phase>,
}

impl EnemyDefinition {
    pub fn get_phase(&self, id: &PhaseId) -> Option<&Phase> {
        self.phases.iter().find(|p| &p.id == id)
    }
}

// ============================================================================
// RUNTIME COMPONENTS
// ============================================================================

/// Attached to each enemy entity
#[derive(Component)]
pub struct EnemyBehavior {
    pub definition: EnemyDefinition,
    pub current_phase: PhaseId,
    pub phase_timer: Timer,
}

/// Ré-export : la santé est centralisée dans `physic/health.rs` et partagée
/// entre tous les ennemis, le joueur, les astéroïdes.
pub use crate::physic::health::Health;

/// Event for external phase triggers
#[derive(Event)]
pub struct PhaseEvent {
    pub entity: Entity,
    pub event_name: &'static str,
}

/// Global events that affect all enemies
#[derive(Event)]
pub struct GlobalPhaseEvent {
    pub event_name: &'static str,
}
// ============================================================================
// SYSTEMS
// ============================================================================

/// Tick les timers, évalue les transitions et collecte les entités qui viennent
/// de changer de phase (pour déclencher leur `on_enter` dans un deuxième temps).
///
/// NOTE : ce système est désormais exclusif (`&mut World`) pour pouvoir exécuter
/// les `on_enter` immédiatement après le changement de phase.
pub fn phase_transition_system(world: &mut World) {
    let dt = world.resource::<Time>().delta();

    // Lire les événements ciblés et globaux
    let targeted: Vec<(Entity, &'static str)> = world
        .resource_mut::<Events<PhaseEvent>>()
        .drain()
        .map(|e| (e.entity, e.event_name))
        .collect();
    let globals: Vec<&'static str> = world
        .resource_mut::<Events<GlobalPhaseEvent>>()
        .drain()
        .map(|e| e.event_name)
        .collect();

    // Collecter (entity, next_phase_id) pour les transitions déclenchées.
    let mut transitions_to_apply: Vec<(Entity, PhaseId)> = Vec::new();

    {
        let mut query = world.query::<(Entity, &mut EnemyBehavior, Option<&Health>)>();
        for (entity, mut enemy, health) in query.iter_mut(world) {
            enemy.phase_timer.tick(dt);

            let Some(current_phase) =
                enemy.definition.get_phase(&enemy.current_phase).cloned()
            else {
                continue;
            };

            // Trier par priorité décroissante
            let mut transitions = current_phase.transitions.clone();
            transitions.sort_by_key(|t| -t.priority);

            for transition in transitions {
                let should_transition = match &transition.trigger {
                    TransitionTrigger::Timer(duration) => {
                        enemy.phase_timer.elapsed() >= *duration
                    }
                    TransitionTrigger::HealthBelow(threshold) => {
                        health.map_or(false, |h| h.fraction() < *threshold)
                    }
                    TransitionTrigger::HealthAbove(threshold) => {
                        health.map_or(false, |h| h.fraction() > *threshold)
                    }
                    TransitionTrigger::Event(name) => {
                        targeted
                            .iter()
                            .any(|(e, n)| *e == entity && n == name)
                            || globals.contains(name)
                    }
                    TransitionTrigger::Custom(_predicate) => {
                        // NOTE : Custom prend `&World`. Dans ce bloc `world` est
                        // déjà emprunté par la query. On évalue les Custom en
                        // deuxième passe si besoin (non implémenté ici — à
                        // remplacer par un mécanisme plus robuste si utilisé).
                        false
                    }
                };

                if should_transition {
                    enemy.current_phase = transition.target_phase.clone();
                    enemy.phase_timer.reset();
                    transitions_to_apply.push((entity, transition.target_phase.clone()));
                    break;
                }
            }
        }
    }

    // Exécuter les on_enter des phases nouvellement entrées
    for (entity, phase_id) in transitions_to_apply {
        let on_enter = {
            let enemy = world.get::<EnemyBehavior>(entity);
            enemy.and_then(|e| {
                e.definition
                    .get_phase(&phase_id)
                    .and_then(|p| p.on_enter.clone())
            })
        };
        if let Some(behavior) = on_enter {
            behavior.0.execute(entity, world);
        }
    }
}

pub fn behavior_execution_system(world: &mut World) {
    // Collect entities and their current behaviors first
    let to_execute: Vec<(Entity, BehaviorBox)> = world
        .query::<(Entity, &EnemyBehavior)>()
        .iter(world)
        .filter_map(|(entity, enemy)| {
            enemy.definition
                .get_phase(&enemy.current_phase)
                .map(|phase| (entity, phase.behavior.clone()))
        })
        .collect();

    // Execute behaviors with mutable world access
    for (entity, behavior) in to_execute {
        behavior.0.execute(entity, world);
    }
}
// ============================================================================
// HELPERS PUBLICS (DSL pour construire des définitions lisibles)
// ============================================================================

/// Crée un `BehaviorBox` depuis un behavior quelconque.
pub fn b<B: Behavior>(behavior: B) -> BehaviorBox {
    BehaviorBox::new(behavior)
}

/// Combinateur Sequence : exécute les behaviors dans l'ordre à chaque frame.
pub fn seq(behaviors: Vec<BehaviorBox>) -> BehaviorBox {
    BehaviorBox::new(Sequence(behaviors))
}

/// Combinateur RandomChoice : exécute un behavior au hasard à chaque frame.
pub fn random(behaviors: Vec<BehaviorBox>) -> BehaviorBox {
    BehaviorBox::new(RandomChoice(behaviors))
}

/// Behavior "noop" — utile comme placeholder dans un on_enter ou une phase.
pub struct Noop;
impl Behavior for Noop {
    fn execute(&self, _entity: Entity, _world: &mut World) {}
    fn name(&self) -> &'static str { "Noop" }
}

// ============================================================================
// SPAWN + PLUGIN
// ============================================================================

/// Spawne un ennemi piloté par `EnemyDefinition`. Le Transform et les
/// composants spécifiques (sprite, Enemy, drops, etc.) sont ajoutés par
/// le code appelant en chaînant `.insert(...)` sur le `EntityCommands` renvoyé
/// si besoin. Ici on installe seulement le cœur : `EnemyBehavior` + `Health`
/// + Transform.
///
/// Déclenche aussi l'éventuel `on_enter` de la phase initiale via un
/// EventWriter interne — pour l'instant il est déclenché par le premier
/// tick de `phase_transition_system` au frame suivant.
pub fn spawn_enemy<'a>(
    commands: &'a mut Commands,
    definition: EnemyDefinition,
    position: Vec3,
    health: i32,
) -> bevy::ecs::system::EntityCommands<'a> {
    let initial_phase = definition.initial_phase.clone();
    commands.spawn((
        EnemyBehavior {
            definition,
            current_phase: initial_phase,
            phase_timer: Timer::from_seconds(0.0, TimerMode::Once),
        },
        Health::new(health),
        TransformBundle::from_transform(Transform::from_translation(position)),
    ))
}

pub struct BehaviorPlugin;

impl Plugin for BehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<PhaseEvent>()
            .add_event::<GlobalPhaseEvent>()
            .add_systems(
                Update,
                (phase_transition_system, behavior_execution_system).chain(),
            );
    }
}

