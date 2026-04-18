//! Framework data-driven pour les ennemis : `Phase`, `Transition`, `Behavior`.
//!
//! **Attention** : le composant `Enemy` (dans `enemy/enemy.rs`) porte désormais
//! le state (`definition`, `current_phase`, `phase_timer`). Ce module ne fournit
//! que les TYPES du framework et les SYSTÈMES qui pilotent les transitions.

use bevy::prelude::*;
use std::time::Duration;

use crate::physic::health::Health;

// Ré-export pour que les modules qui importent `crate::enemy::system::Health`
// continuent de compiler.
pub use crate::physic::health::Health as HealthAlias;

// ============================================================================
// BEHAVIOR PRIMITIVES
// ============================================================================

/// Un `Behavior` est un trait object qui peut s'exécuter chaque frame.
/// Les behaviors sont sans état — tout l'état est dans le `World` (composants).
pub trait Behavior: Send + Sync + 'static {
    fn execute(&self, entity: Entity, world: &mut World);
    fn name(&self) -> &'static str;
}

/// Wrapper type-erased, clonable à coût constant (Arc).
#[derive(Clone)]
pub struct BehaviorBox(pub std::sync::Arc<dyn Behavior>);

impl BehaviorBox {
    pub fn new<B: Behavior>(behavior: B) -> Self {
        Self(std::sync::Arc::new(behavior))
    }
}

// ============================================================================
// BEHAVIOR COMBINATORS
// ============================================================================

/// Exécute les behaviors l'un après l'autre chaque frame.
pub struct Sequence(pub Vec<BehaviorBox>);

impl Behavior for Sequence {
    fn execute(&self, entity: Entity, world: &mut World) {
        for behavior in &self.0 {
            behavior.0.execute(entity, world);
        }
    }
    fn name(&self) -> &'static str {
        "Sequence"
    }
}

/// Choisit un behavior au hasard à chaque frame.
pub struct RandomChoice(pub Vec<BehaviorBox>);

impl Behavior for RandomChoice {
    fn execute(&self, entity: Entity, world: &mut World) {
        if self.0.is_empty() {
            return;
        }
        let idx = fastrand::usize(0..self.0.len());
        self.0[idx].0.execute(entity, world);
    }
    fn name(&self) -> &'static str {
        "RandomChoice"
    }
}

/// Choix pondéré.
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
    fn name(&self) -> &'static str {
        "WeightedChoice"
    }
}

// ============================================================================
// PHASES
// ============================================================================

/// Identifiant de phase (chaîne statique — bon compromis debug/performance).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PhaseId(pub &'static str);

/// Une phase = un behavior per-frame + des transitions sortantes.
#[derive(Clone)]
pub struct Phase {
    pub id: PhaseId,
    /// Behavior one-shot exécuté quand on entre dans cette phase.
    pub on_enter: Option<BehaviorBox>,
    /// Behavior exécuté à chaque frame tant que l'entité est dans cette phase.
    pub behavior: BehaviorBox,
    /// Transitions possibles vers d'autres phases.
    pub transitions: Vec<Transition>,
    /// Si `true`, l'ennemi est invulnérable aux projectiles pendant cette phase.
    /// Utilisé pour `entering`, `transitioning`, `dying`, `dead`.
    pub invulnerable: bool,
}

impl Phase {
    /// Phase vulnérable, sans on_enter.
    pub fn new(id: PhaseId, behavior: BehaviorBox, transitions: Vec<Transition>) -> Self {
        Self {
            id,
            on_enter: None,
            behavior,
            transitions,
            invulnerable: false,
        }
    }

    pub fn with_on_enter(mut self, on_enter: BehaviorBox) -> Self {
        self.on_enter = Some(on_enter);
        self
    }

    /// Marque la phase comme invulnérable (projectiles traversent sans dégâts).
    pub fn invulnerable(mut self) -> Self {
        self.invulnerable = true;
        self
    }
}

#[derive(Clone)]
pub enum TransitionTrigger {
    Timer(Duration),
    HealthBelow(f32),
    HealthAbove(f32),
    Event(&'static str),
    Custom(std::sync::Arc<dyn Fn(Entity, &World) -> bool + Send + Sync>),
}

#[derive(Clone)]
pub struct Transition {
    pub trigger: TransitionTrigger,
    pub target_phase: PhaseId,
    pub priority: i32,
}

// ============================================================================
// ENEMY DEFINITION
// ============================================================================

/// Définition déclarative d'un archétype d'ennemi : phase initiale + liste
/// de phases. Attachée à chaque entité via `Enemy.definition`.
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
// EVENTS
// ============================================================================

/// Event déclencheur de phase, ciblé sur une entité spécifique.
#[derive(Event)]
pub struct PhaseEvent {
    pub entity: Entity,
    pub event_name: &'static str,
}

/// Event déclencheur de phase, global (toutes les entités écoutent).
#[derive(Event)]
pub struct GlobalPhaseEvent {
    pub event_name: &'static str,
}

// ============================================================================
// SYSTEMS
// ============================================================================

// Import concret du composant Enemy (défini dans enemy.rs) — chemin absolu
// pour éviter une circularité de déclarations en tête de fichier.
use crate::enemy::enemy::Enemy;

/// Tick le phase_timer, évalue les transitions et déclenche l'`on_enter`
/// des phases cibles.
pub fn phase_transition_system(world: &mut World) {
    let dt = world.resource::<Time>().delta();

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

    let mut transitions_to_apply: Vec<(Entity, PhaseId)> = Vec::new();

    {
        let mut query = world.query::<(Entity, &mut Enemy, Option<&Health>)>();
        for (entity, mut enemy, health) in query.iter_mut(world) {
            enemy.phase_timer.tick(dt);

            let Some(current_phase) = enemy.definition.get_phase(&enemy.current_phase).cloned()
            else {
                continue;
            };

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
                        targeted.iter().any(|(e, n)| *e == entity && n == name)
                            || globals.contains(name)
                    }
                    TransitionTrigger::Custom(_) => false, // non évalué ici
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
            let enemy = world.get::<Enemy>(entity);
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

/// Exécute le behavior de la phase courante pour chaque ennemi.
pub fn behavior_execution_system(world: &mut World) {
    let to_execute: Vec<(Entity, BehaviorBox)> = world
        .query::<(Entity, &Enemy)>()
        .iter(world)
        .filter_map(|(entity, enemy)| {
            enemy
                .definition
                .get_phase(&enemy.current_phase)
                .map(|phase| (entity, phase.behavior.clone()))
        })
        .collect();

    for (entity, behavior) in to_execute {
        behavior.0.execute(entity, world);
    }
}

// ============================================================================
// HELPERS DSL
// ============================================================================

pub fn b<B: Behavior>(behavior: B) -> BehaviorBox {
    BehaviorBox::new(behavior)
}

pub fn seq(behaviors: Vec<BehaviorBox>) -> BehaviorBox {
    BehaviorBox::new(Sequence(behaviors))
}

pub fn random(behaviors: Vec<BehaviorBox>) -> BehaviorBox {
    BehaviorBox::new(RandomChoice(behaviors))
}

pub struct Noop;
impl Behavior for Noop {
    fn execute(&self, _entity: Entity, _world: &mut World) {}
    fn name(&self) -> &'static str {
        "Noop"
    }
}

// ============================================================================
// PLUGIN (framework events uniquement ; les systèmes sont branchés par EnemyPlugin)
// ============================================================================

pub struct BehaviorFrameworkPlugin;

impl Plugin for BehaviorFrameworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<PhaseEvent>().add_event::<GlobalPhaseEvent>();
    }
}
