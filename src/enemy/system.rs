use bevy::prelude::*;
use rand::seq::SliceRandom;
use std::time::Duration;
use rand::distributions::WeightedIndex;
use rand::prelude::*;
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
        if let Some(behavior) = self.0.choose(&mut rand::thread_rng()) {
            behavior.0.execute(entity, world);
        }
    }
    fn name(&self) -> &'static str { "RandomChoice" }
}

/// Weighted random selection
pub struct WeightedChoice(pub Vec<(f32, BehaviorBox)>);

impl Behavior for WeightedChoice {
    fn execute(&self, entity: Entity, world: &mut World) {
        
        
        let weights: Vec<f32> = self.0.iter().map(|(w, _)| *w).collect();
        if let Ok(dist) = WeightedIndex::new(&weights) {
            let idx = dist.sample(&mut rand::thread_rng());
            self.0[idx].1.0.execute(entity, world);
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
    pub behavior: BehaviorBox,
    pub transitions: Vec<Transition>,
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

/// Health component for health-based transitions
#[derive(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    pub fn fraction(&self) -> f32 {
        self.current / self.max
    }
}

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

pub fn phase_transition_system(
    mut enemies: Query<(Entity, &mut EnemyBehavior, Option<&Health>)>,
    time: Res<Time>,
    mut phase_events: EventReader<PhaseEvent>,
    mut global_events: EventReader<GlobalPhaseEvent>,
    world: &World,
) {
    // Collect events
    let targeted_events: Vec<_> = phase_events.read().collect();
    let global_event_names: Vec<_> = global_events.read().map(|e| e.event_name).collect();

    for (entity, mut enemy, health) in enemies.iter_mut() {
        enemy.phase_timer.tick(time.delta());

        let Some(current_phase) = enemy.definition.get_phase(&enemy.current_phase).cloned() 
            else { continue };

        // Check transitions in priority order
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
                    targeted_events.iter().any(|e| e.entity == entity && e.event_name == *name)
                        || global_event_names.contains(name)
                }
                TransitionTrigger::Custom(predicate) => {
                    predicate(entity, world)
                }
            };

            if should_transition {
                enemy.current_phase = transition.target_phase.clone();
                enemy.phase_timer.reset();
                break;
            }
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
// CONCRETE BEHAVIORS (your game-specific actions)
// ============================================================================

pub struct MoveTowardPlayer { pub speed: f32 }
pub struct CirclePlayer { pub radius: f32, pub speed: f32 }
pub struct ShootAtPlayer { pub projectile_speed: f32 }
pub struct SprayBullets { pub count: u32, pub spread: f32 }
pub struct Dash { pub distance: f32 }
pub struct Teleport;
pub struct SpawnMinions { pub count: u32 }
pub struct Enrage; // Could buff stats, change visuals, etc.

// Implement Behavior for each...
impl Behavior for MoveTowardPlayer {
    fn execute(&self, entity: Entity, world: &mut World) {
        // Your movement logic here
    }
    fn name(&self) -> &'static str { "MoveTowardPlayer" }
}

// ... (similar impls for others)

// ============================================================================
// ENEMY DEFINITIONS (the declarative DSL)
// ============================================================================

/// Helper macros/functions for cleaner definitions
fn seq(behaviors: Vec<BehaviorBox>) -> BehaviorBox {
    BehaviorBox::new(Sequence(behaviors))
}

fn random(behaviors: Vec<BehaviorBox>) -> BehaviorBox {
    BehaviorBox::new(RandomChoice(behaviors))
}

fn b<B: Behavior>(behavior: B) -> BehaviorBox {
    BehaviorBox::new(behavior)
}

pub fn skeleton_warrior() -> EnemyDefinition {
    EnemyDefinition {
        name: "Skeleton Warrior",
        initial_phase: PhaseId("aggressive"),
        phases: vec![
            Phase {
                id: PhaseId("aggressive"),
                behavior: seq(vec![
                    b(MoveTowardPlayer { speed: 100.0 }),
                    b(ShootAtPlayer { projectile_speed: 200.0 }),
                ]),
                transitions: vec![
                    Transition {
                        trigger: TransitionTrigger::HealthBelow(0.3),
                        target_phase: PhaseId("desperate"),
                        priority: 10,
                    },
                ],
            },
            Phase {
                id: PhaseId("desperate"),
                behavior: random(vec![
                    b(Dash { distance: 150.0 }),
                    b(SprayBullets { count: 8, spread: 45.0 }),
                ]),
                transitions: vec![
                    Transition {
                        trigger: TransitionTrigger::HealthAbove(0.5),
                        target_phase: PhaseId("aggressive"),
                        priority: 0,
                    },
                ],
            },
        ],
    }
}

pub fn boss_demon() -> EnemyDefinition {
    EnemyDefinition {
        name: "Demon Boss",
        initial_phase: PhaseId("phase1"),
        phases: vec![
            Phase {
                id: PhaseId("phase1"),
                behavior: seq(vec![
                    b(CirclePlayer { radius: 200.0, speed: 50.0 }),
                    random(vec![
                        b(ShootAtPlayer { projectile_speed: 150.0 }),
                        b(SpawnMinions { count: 2 }),
                    ]),
                ]),
                transitions: vec![
                    Transition {
                        trigger: TransitionTrigger::HealthBelow(0.66),
                        target_phase: PhaseId("phase2"),
                        priority: 10,
                    },
                    Transition {
                        trigger: TransitionTrigger::Timer(Duration::from_secs(30)),
                        target_phase: PhaseId("enraged"),
                        priority: 5,
                    },
                ],
            },
            Phase {
                id: PhaseId("phase2"),
                behavior: seq(vec![
                    b(Teleport),
                    b(SprayBullets { count: 16, spread: 360.0 }),
                ]),
                transitions: vec![
                    Transition {
                        trigger: TransitionTrigger::HealthBelow(0.33),
                        target_phase: PhaseId("phase3"),
                        priority: 10,
                    },
                ],
            },
            Phase {
                id: PhaseId("phase3"),
                behavior: seq(vec![
                    b(Enrage),
                    b(Dash { distance: 300.0 }),
                    b(SprayBullets { count: 24, spread: 360.0 }),
                    b(SpawnMinions { count: 4 }),
                ]),
                transitions: vec![
                    // No transitions - fight to the death
                ],
            },
            Phase {
                id: PhaseId("enraged"),
                // Reuses phase3 behavior but accessible via timer
                behavior: seq(vec![
                    b(Enrage),
                    b(Dash { distance: 300.0 }),
                    b(SprayBullets { count: 24, spread: 360.0 }),
                ]),
                transitions: vec![
                    Transition {
                        trigger: TransitionTrigger::HealthBelow(0.33),
                        target_phase: PhaseId("phase3"),
                        priority: 10,
                    },
                ],
            },
        ],
    }
}
pub fn spawn_enemy(commands: &mut Commands, definition: EnemyDefinition, position: Vec3) {
    let initial_phase = definition.initial_phase.clone();
    
    commands.spawn((
        EnemyBehavior {
            definition,
            current_phase: initial_phase,
            phase_timer: Timer::from_seconds(0.0, TimerMode::Once),
        },
        Health { current: 100.0, max: 100.0 },
        Transform::from_translation(position),
        // ... other components
    ));
}

