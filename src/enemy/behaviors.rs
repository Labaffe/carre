//! Bibliothèque de `Behavior`s réutilisables par n'importe quel ennemi.
//!
//! Chaque behavior est un type `Clone + Send + Sync + 'static` qui implémente
//! le trait `Behavior` défini dans `enemy/system.rs`. Les behaviors sont :
//! - **stateless** : aucun état interne. Tout l'état vit dans des composants
//!   attachés à l'entité (via `&mut World`).
//! - **composables** : on peut les combiner avec `seq()`, `random()` ou les
//!   utiliser comme `on_enter` d'une phase.
//!
//! ## Catégories
//! - **Mouvement** : LinearMove, FallDown, RushTowardPlayer, PatrolSine,
//!   DriftFloat, EnterFromOffscreen, IntroSpiral
//! - **Visée & tir** : AimAtPlayer, PeriodicShoot, SprayBurst
//! - **Visuels** : AnimateFrames, FlashWhite, SetScale, ShakeAround
//! - **Utilitaires** : PlaySound, SpawnFromDifficulty, DespawnSelf,
//!   EmitDropEvent

use bevy::prelude::*;

use crate::enemy::enemy::Enemy;
use crate::enemy::system::Behavior;
use crate::game_manager::difficulty::{Difficulty, SpawnPosition};
use crate::item::item::{DropEvent, DropTable, ItemType};
use crate::player::player::Player;
use crate::weapon::projectile::{spawn_projectile, ProjectileSpawn, ProjectileSprite, Team};
use crate::weapon::weapon::HitboxShape;

// ═══════════════════════════════════════════════════════════════════════
//  Mouvement
// ═══════════════════════════════════════════════════════════════════════

/// Déplace l'entité à vitesse constante (Vec3).
pub struct LinearMove {
    pub velocity: Vec3,
}

impl Behavior for LinearMove {
    fn execute(&self, entity: Entity, world: &mut World) {
        let dt = world.resource::<Time>().delta_seconds();
        if let Some(mut tr) = world.get_mut::<Transform>(entity) {
            tr.translation += self.velocity * dt;
        }
    }
    fn name(&self) -> &'static str {
        "LinearMove"
    }
}

/// Chute verticale — raccourci pour `LinearMove { velocity: -speed * Y }`.
pub struct FallDown {
    pub speed: f32,
}

impl Behavior for FallDown {
    fn execute(&self, entity: Entity, world: &mut World) {
        let dt = world.resource::<Time>().delta_seconds();
        if let Some(mut tr) = world.get_mut::<Transform>(entity) {
            tr.translation.y -= self.speed * dt;
        }
    }
    fn name(&self) -> &'static str {
        "FallDown"
    }
}

/// Direction figée pour un rush (insérée par on_enter, lue par RushTowardPlayer).
#[derive(Component)]
pub struct RushDirection(pub Vec2);

/// Fonce en ligne droite. La direction est prise depuis le composant `RushDirection`
/// (à poser via `SetRushDirection` en on_enter).
pub struct RushMove {
    pub speed: f32,
}

impl Behavior for RushMove {
    fn execute(&self, entity: Entity, world: &mut World) {
        let dt = world.resource::<Time>().delta_seconds();
        let dir = world.get::<RushDirection>(entity).map(|d| d.0);
        if let Some(dir) = dir {
            if let Some(mut tr) = world.get_mut::<Transform>(entity) {
                tr.translation.x += dir.x * self.speed * dt;
                tr.translation.y += dir.y * self.speed * dt;
            }
        }
    }
    fn name(&self) -> &'static str {
        "RushMove"
    }
}

/// On-enter behavior : calcule la direction vers le joueur et insère
/// `RushDirection` sur l'entité.
pub struct SetRushDirection;

impl Behavior for SetRushDirection {
    fn execute(&self, entity: Entity, world: &mut World) {
        let my_pos = world
            .get::<Transform>(entity)
            .map(|t| t.translation.truncate());
        let player_pos = world
            .query_filtered::<&Transform, With<Player>>()
            .get_single(world)
            .ok()
            .map(|t| t.translation.truncate());

        if let (Some(m), Some(p)) = (my_pos, player_pos) {
            let dir = (p - m).normalize_or_zero();
            world.entity_mut(entity).insert(RushDirection(if dir == Vec2::ZERO {
                Vec2::new(0.0, -1.0)
            } else {
                dir
            }));
        }
    }
    fn name(&self) -> &'static str {
        "SetRushDirection"
    }
}

/// Patrouille sinusoïdale : X avance à `speed_x` avec rebond aux bords,
/// Y suit une sinusoïde dépendant du `phase_timer`.
pub struct PatrolSine {
    pub speed_x: f32,
    /// Amplitude verticale en fraction de (window_half_h - margin).
    pub sine_amp_ratio: f32,
    pub sine_freq: f32,
    pub margin: f32,
}

impl Behavior for PatrolSine {
    fn execute(&self, entity: Entity, world: &mut World) {
        let dt = world.resource::<Time>().delta_seconds();
        let t = world
            .get::<Enemy>(entity)
            .map(|e| e.phase_timer.elapsed().as_secs_f32())
            .unwrap_or(0.0);

        let (half_w, half_h) = query_window_half(world)
            .map(|(w, h)| (w - self.margin, h - self.margin))
            .unwrap_or((640.0, 360.0));

        if let Some(mut tr) = world.get_mut::<Transform>(entity) {
            tr.translation.x = (tr.translation.x + self.speed_x * dt).clamp(-half_w, half_w);
            tr.translation.y = (t * self.sine_freq).sin() * half_h * self.sine_amp_ratio;
        }
    }
    fn name(&self) -> &'static str {
        "PatrolSine"
    }
}

/// Dérive ample (style mothership) : deux oscillations superposées.
pub struct DriftFloat {
    pub main_amp_x: f32,
    pub main_freq_x: f32,
    pub minor_amp_y: f32,
    pub minor_freq_y: f32,
    pub anchor: Vec3,
}

impl Behavior for DriftFloat {
    fn execute(&self, entity: Entity, world: &mut World) {
        let t = world
            .get::<Enemy>(entity)
            .map(|e| e.phase_timer.elapsed().as_secs_f32())
            .unwrap_or(0.0);
        let dx = (t * self.main_freq_x).sin() * self.main_amp_x;
        let dy = (t * self.minor_freq_y).cos() * self.minor_amp_y;
        if let Some(mut tr) = world.get_mut::<Transform>(entity) {
            tr.translation.x = self.anchor.x + dx;
            tr.translation.y = self.anchor.y + dy;
        }
    }
    fn name(&self) -> &'static str {
        "DriftFloat"
    }
}

/// Animation d'entrée linéaire entre deux points, avec ease-out quadratique.
pub struct EnterFromOffscreen {
    pub from: Vec3,
    pub to: Vec3,
    pub duration: f32,
}

impl Behavior for EnterFromOffscreen {
    fn execute(&self, entity: Entity, world: &mut World) {
        let t = world
            .get::<Enemy>(entity)
            .map(|e| (e.phase_timer.elapsed().as_secs_f32() / self.duration).clamp(0.0, 1.0))
            .unwrap_or(1.0);
        let eased = 1.0 - (1.0 - t).powi(2);
        let pos = self.from.lerp(self.to, eased);
        if let Some(mut tr) = world.get_mut::<Transform>(entity) {
            tr.translation = pos;
        }
    }
    fn name(&self) -> &'static str {
        "EnterFromOffscreen"
    }
}

/// Intro en spirale avec scaling (style boss).
pub struct IntroSpiral {
    pub duration: f32,
    pub target_y: f32,
    pub start_scale: f32,
    pub end_scale: f32,
    pub turns: f32,
    pub radius: f32,
    pub from_y: f32,
}

impl Behavior for IntroSpiral {
    fn execute(&self, entity: Entity, world: &mut World) {
        let t = world
            .get::<Enemy>(entity)
            .map(|e| (e.phase_timer.elapsed().as_secs_f32() / self.duration).clamp(0.0, 1.0))
            .unwrap_or(1.0);
        let eased = 1.0 - (1.0 - t).powi(2);

        let angle = t * self.turns * std::f32::consts::TAU;
        let radius = self.radius * (1.0 - eased);
        let dx = angle.cos() * radius;
        let y = self.from_y + (self.target_y - self.from_y) * eased;
        let scale = self.start_scale + (self.end_scale - self.start_scale) * eased;

        if let Some(mut tr) = world.get_mut::<Transform>(entity) {
            tr.translation.x = dx;
            tr.translation.y = y;
            tr.scale = Vec3::splat(scale);
        }
    }
    fn name(&self) -> &'static str {
        "IntroSpiral"
    }
}

/// Despawn quand l'entité sort des bornes de l'écran (+ marge).
pub struct DespawnIfOffscreen {
    pub margin: f32,
}

impl Behavior for DespawnIfOffscreen {
    fn execute(&self, entity: Entity, world: &mut World) {
        let pos = world
            .get::<Transform>(entity)
            .map(|t| t.translation)
            .unwrap_or(Vec3::ZERO);
        let (half_w, half_h) = query_window_half(world).unwrap_or((640.0, 360.0));
        if pos.x.abs() > half_w + self.margin || pos.y.abs() > half_h + self.margin {
            if world.get_entity(entity).is_some() {
                world.despawn(entity);
            }
        }
    }
    fn name(&self) -> &'static str {
        "DespawnIfOffscreen"
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Visée & Tir
// ═══════════════════════════════════════════════════════════════════════

/// Mode de visée utilisé par les behaviors de tir.
#[derive(Clone, Copy)]
pub enum AimMode {
    /// Tir droit devant (axe local +Y du sprite).
    Forward,
    /// Verrouille le joueur.
    AtPlayer,
    /// Direction fixée.
    Fixed(Vec2),
}

impl AimMode {
    /// Résout le mode en direction unitaire. Utilise la rotation courante de
    /// l'entité pour `Forward` et la position du joueur pour `AtPlayer`.
    fn resolve(&self, entity: Entity, world: &World) -> Vec2 {
        match self {
            AimMode::Forward => {
                let rot = world
                    .get::<Transform>(entity)
                    .map(|t| t.rotation)
                    .unwrap_or(Quat::IDENTITY);
                let local = Vec3::new(0.0, 1.0, 0.0);
                let v = rot.mul_vec3(local);
                Vec2::new(v.x, v.y).normalize_or_zero()
            }
            AimMode::AtPlayer => {
                let my_pos = world
                    .get::<Transform>(entity)
                    .map(|t| t.translation.truncate());
                let player = world_find_player_pos(world);
                if let (Some(m), Some(p)) = (my_pos, player) {
                    (p - m).normalize_or_zero()
                } else {
                    Vec2::new(0.0, -1.0)
                }
            }
            AimMode::Fixed(v) => v.normalize_or_zero(),
        }
    }
}

/// Définition d'un projectile à spawner (version simple sans la couleur/sprite).
#[derive(Clone)]
pub struct ProjectileDef {
    pub sprite: ProjectileSprite,
    pub speed: f32,
    pub hitbox: HitboxShape,
    pub damage: i32,
    pub death_folder: Option<&'static str>,
}

/// Composant timer de tir, inséré automatiquement par `PeriodicShoot`.
#[derive(Component)]
pub struct FireTimer(pub Timer);

/// Tire un projectile à intervalle régulier.
pub struct PeriodicShoot {
    pub projectile: ProjectileDef,
    pub interval: f32,
    pub aim: AimMode,
    pub sound: Option<&'static str>,
}

impl Behavior for PeriodicShoot {
    fn execute(&self, entity: Entity, world: &mut World) {
        // Init du timer s'il n'existe pas encore
        if world.get::<FireTimer>(entity).is_none() {
            world
                .entity_mut(entity)
                .insert(FireTimer(Timer::from_seconds(self.interval, TimerMode::Repeating)));
            return; // attend le prochain frame pour tirer
        }

        let dt = world.resource::<Time>().delta();
        let just_finished = {
            let mut timer = world.get_mut::<FireTimer>(entity).unwrap();
            timer.0.tick(dt);
            timer.0.just_finished()
        };
        if !just_finished {
            return;
        }

        // Résoudre direction et position
        let dir = self.aim.resolve(entity, world);
        let my_pos = world
            .get::<Transform>(entity)
            .map(|t| t.translation)
            .unwrap_or(Vec3::ZERO);

        // Spawn via commands / asset_server (via world)
        let sprite = self.projectile.sprite.clone();
        let hitbox = self.projectile.hitbox.clone();
        let speed = self.projectile.speed;
        let damage = self.projectile.damage;
        let death = self.projectile.death_folder;
        let sound = self.sound;

        let mut commands_queue = bevy::ecs::system::CommandQueue::default();
        {
            let asset_server = world.resource::<AssetServer>().clone();
            let mut commands = Commands::new(&mut commands_queue, world);
            spawn_projectile(
                &mut commands,
                &asset_server,
                ProjectileSpawn {
                    position: my_pos + Vec3::new(0.0, 0.0, 0.1),
                    direction: dir,
                    speed,
                    hitbox,
                    team: Team::Enemy,
                    damage,
                    sprite,
                    death_folder: death,
                },
            );
            if let Some(path) = sound {
                let src = asset_server.load(path);
                commands.spawn(AudioBundle {
                    source: src,
                    settings: PlaybackSettings::DESPAWN,
                });
            }
        }
        commands_queue.apply(world);
    }
    fn name(&self) -> &'static str {
        "PeriodicShoot"
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Visuels
// ═══════════════════════════════════════════════════════════════════════

/// Cycle les frames préchargées dans un dossier. Le premier frame est affiché
/// au spawn, puis on cycle à `fps` images/seconde. Les handles sont fournis via
/// la ressource `FrameAtlas<T>` (cf. système `frame_atlas.rs` à créer) — pour
/// l'instant on utilise un composant local qui stocke les handles.
#[derive(Component)]
pub struct FrameCycle {
    pub frames: Vec<Handle<Image>>,
    pub fps: f32,
    pub timer: Timer,
    pub current: usize,
}

impl FrameCycle {
    pub fn new(frames: Vec<Handle<Image>>, fps: f32) -> Self {
        Self {
            timer: Timer::from_seconds(1.0 / fps.max(0.1), TimerMode::Repeating),
            frames,
            fps,
            current: 0,
        }
    }
}

/// Behavior qui fait avancer un `FrameCycle`. À utiliser en séquence avec le
/// mouvement (ex : seq([PatrolSine, CycleFramesBehavior])).
pub struct CycleFrames;

impl Behavior for CycleFrames {
    fn execute(&self, entity: Entity, world: &mut World) {
        let dt = world.resource::<Time>().delta();
        let new_frame = {
            let Some(mut cycle) = world.get_mut::<FrameCycle>(entity) else {
                return;
            };
            cycle.timer.tick(dt);
            if !cycle.timer.just_finished() {
                return;
            }
            let len = cycle.frames.len();
            if len == 0 {
                return;
            }
            cycle.current = (cycle.current + 1) % len;
            cycle.frames[cycle.current].clone()
        };
        if let Some(mut handle) = world.get_mut::<Handle<Image>>(entity) {
            *handle = new_frame;
        }
    }
    fn name(&self) -> &'static str {
        "CycleFrames"
    }
}

/// Flash blanc durant `duration` secondes après le début de la phase.
pub struct FlashWhite {
    pub duration: f32,
}

impl Behavior for FlashWhite {
    fn execute(&self, entity: Entity, world: &mut World) {
        let elapsed = world
            .get::<Enemy>(entity)
            .map(|e| e.phase_timer.elapsed().as_secs_f32())
            .unwrap_or(0.0);
        if let Some(mut sprite) = world.get_mut::<Sprite>(entity) {
            if elapsed < self.duration {
                sprite.color = Color::rgba(100.0, 100.0, 100.0, 1.0);
            } else {
                sprite.color = Color::WHITE;
            }
        }
    }
    fn name(&self) -> &'static str {
        "FlashWhite"
    }
}

/// Applique un tremblement random (modifie Transform à chaque frame).
pub struct ShakeAround {
    pub amplitude: f32,
}

impl Behavior for ShakeAround {
    fn execute(&self, entity: Entity, world: &mut World) {
        let dx = (fastrand::f32() - 0.5) * 2.0 * self.amplitude;
        let dy = (fastrand::f32() - 0.5) * 2.0 * self.amplitude;
        if let Some(mut tr) = world.get_mut::<Transform>(entity) {
            tr.translation.x += dx;
            tr.translation.y += dy;
        }
    }
    fn name(&self) -> &'static str {
        "ShakeAround"
    }
}

/// Met l'échelle à une valeur précise (one-shot pratique via on_enter).
pub struct SetScale(pub f32);

impl Behavior for SetScale {
    fn execute(&self, entity: Entity, world: &mut World) {
        if let Some(mut tr) = world.get_mut::<Transform>(entity) {
            tr.scale = Vec3::splat(self.0);
        }
    }
    fn name(&self) -> &'static str {
        "SetScale"
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Utilitaires
// ═══════════════════════════════════════════════════════════════════════

/// Joue un son one-shot (à utiliser dans on_enter).
pub struct PlaySound {
    pub path: &'static str,
    pub volume: f32,
}

impl Behavior for PlaySound {
    fn execute(&self, _entity: Entity, world: &mut World) {
        let handle = world.resource::<AssetServer>().load::<AudioSource>(self.path);
        world.spawn(AudioBundle {
            source: handle,
            settings: PlaybackSettings {
                volume: bevy::audio::Volume::new(self.volume),
                ..PlaybackSettings::DESPAWN
            },
        });
    }
    fn name(&self) -> &'static str {
        "PlaySound"
    }
}

/// Injecte une spawn_request dans `Difficulty` pour faire apparaître un autre
/// ennemi (via le système de spawn standard).
pub struct SpawnFromDifficulty {
    pub name: &'static str,
    pub count: usize,
    pub position: SpawnPosition,
}

impl Behavior for SpawnFromDifficulty {
    fn execute(&self, _entity: Entity, world: &mut World) {
        if let Some(mut difficulty) = world.get_resource_mut::<Difficulty>() {
            difficulty
                .spawn_requests
                .push((self.name, self.count, self.position));
        }
    }
    fn name(&self) -> &'static str {
        "SpawnFromDifficulty"
    }
}

/// Despawn l'entité courante.
pub struct DespawnSelf;

impl Behavior for DespawnSelf {
    fn execute(&self, entity: Entity, world: &mut World) {
        if world.get_entity(entity).is_some() {
            world.despawn(entity);
        }
    }
    fn name(&self) -> &'static str {
        "DespawnSelf"
    }
}

/// Émet un `DropEvent` à la position de l'entité (une seule fois, via on_enter
/// typiquement). Utilisé pour lâcher un item à la mort.
pub struct EmitDropEvent {
    pub table: &'static [(ItemType, f32)],
}

impl Behavior for EmitDropEvent {
    fn execute(&self, entity: Entity, world: &mut World) {
        let pos = world
            .get::<Transform>(entity)
            .map(|t| t.translation)
            .unwrap_or(Vec3::ZERO);
        if let Some(mut events) = world.get_resource_mut::<Events<DropEvent>>() {
            events.send(DropEvent {
                position: pos,
                table: self.table,
            });
        }
    }
    fn name(&self) -> &'static str {
        "EmitDropEvent"
    }
}

/// Attache un composant `DropTable` à l'entité (pour que les systèmes
/// existants émettent le drop quand l'entité meurt). Utile en on_enter de
/// spawn.
pub struct AttachDropTable(pub &'static [(ItemType, f32)]);

impl Behavior for AttachDropTable {
    fn execute(&self, entity: Entity, world: &mut World) {
        world.entity_mut(entity).insert(DropTable { drops: self.0 });
    }
    fn name(&self) -> &'static str {
        "AttachDropTable"
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Helpers internes
// ═══════════════════════════════════════════════════════════════════════

fn query_window_half(world: &mut World) -> Option<(f32, f32)> {
    world
        .query::<&Window>()
        .iter(world)
        .next()
        .map(|w| (w.width() / 2.0, w.height() / 2.0))
}

fn world_find_player_pos(world: &World) -> Option<Vec2> {
    // NOTE : world.query() nécessite un World mutable ; ici on passe par
    // `entity_ref` + `World::iter_entities`. Solution simple : le caller
    // passe un world &mut. On accepte &World pour signature cohérente avec
    // AimMode::resolve mais on utilise une query via unsafe cell est overkill.
    // Implémentation : on mock via un composant Player, en supposant qu'une
    // seule entité Player existe.
    for e in world.iter_entities() {
        if e.contains::<Player>() {
            return e.get::<Transform>().map(|t| t.translation.truncate());
        }
    }
    None
}
