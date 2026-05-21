//! Module audio centralisé — un seul point de vérité pour les SFX.
//!
//! Conventions :
//! - L'enum [`Sfx`] liste *tous* les events sonores du jeu. Un même fichier
//!   physique peut être référencé par plusieurs variantes (ex: `t_1.ogg` est
//!   à la fois `UiCountdownBeep` et `MineBeep`) : les variantes représentent
//!   les events *sémantiques*, pas les fichiers.
//! - [`SFX_PATHS`] est la seule place où vivent les chemins d'assets. Swap
//!   `.wav` → `.ogg` = changer une ligne.
//! - Préchargement au démarrage : la [`SfxLibrary`] tient un `Handle` par
//!   variante, ce qui évite tout hitch au premier play.
//! - Pour jouer : utiliser [`SfxPlayer`] comme `SystemParam`. Retourne
//!   l'`EntityCommands` du sample pour permettre `.insert(marker)` quand
//!   c'est utile (ex: `IntroSound` pour le pause sync).
//!
//! La musique garde son ancien système (markers `MusicMain`, `MusicBoss`,
//! etc.) — ce module ne couvre que les SFX.

use bevy::audio::Volume;
use bevy::ecs::system::SystemParam;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

/// Tous les events sonores du jeu. Les variantes sont *sémantiques* — deux
/// variantes peuvent pointer vers le même fichier sans que ce soit redondant.
///
/// Certaines variantes pointent vers des assets présents dans `assets/audio/sfx/`
/// mais ne sont pas encore branchées à un call site dans le code — elles
/// servent de "todo list" pour les futurs wirings (boss, mothership, etc.).
#[allow(dead_code)]
#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub enum Sfx {
    // ─── Player ───
    PlayerShoot,
    PlayerBomb,
    PlayerHurt,
    PlayerDeath,
    // ─── Combat (hors EnemyConfigData) ───
    EnemyHit,
    EnemyDie,
    Explosion,
    // ─── Items ───
    ItemAppear,
    ItemPickup,
    ScoreMilestone,
    // ─── Ennemis spécifiques ───
    KamikazeScream,
    KamikazeLaugh,
    OctopusSound,
    OctopusShoot,
    OctopusRush,
    OctopusDie,
    MineBeep,
    MineExplode,
    BossStart,
    BossStart2,
    BossExplosion,
    BossExplosion2,
    GreenUfoSpawn,
    GreenUfoDeath,
    GatlingShoot,
    MothershipAlarm,
    // ─── UI ───
    UiCountdownBeep,
    UiCountdownReady,
    UiCountdownGo,
    UiPause,
    // ─── Cinématiques / environnement ───
    PlanetLanding,
    ShipArrival,
}

/// Mapping unique variante → chemin d'asset. **Toute** modif de chemin se
/// fait ici. Si tu remplaces un son généré par un vrai, c'est la seule
/// ligne à toucher.
const SFX_PATHS: &[(Sfx, &str)] = &[
    // Player
    (Sfx::PlayerShoot,      "audio/sfx/shoot.wav"),
    (Sfx::PlayerBomb,       "audio/sfx/bomb.ogg"),
    (Sfx::PlayerHurt,       "audio/sfx/hurt.ogg"),
    (Sfx::PlayerDeath,      "audio/sfx/you_died.ogg"),
    // Combat
    (Sfx::EnemyHit,         "audio/sfx/hit.wav"),
    (Sfx::EnemyDie,         "audio/sfx/asteroid_die.ogg"),
    (Sfx::Explosion,        "audio/sfx/explode.wav"),
    // Items
    (Sfx::ItemAppear,       "audio/sfx/level_up.ogg"),
    (Sfx::ItemPickup,       "audio/sfx/pickup.wav"),
    (Sfx::ScoreMilestone,   "audio/sfx/level_up.ogg"),
    // Ennemis
    (Sfx::KamikazeScream,   "audio/sfx/kamikaze_scream.wav"),
    (Sfx::KamikazeLaugh,    "audio/sfx/kamikaze_laugh.wav"),
    (Sfx::OctopusSound,     "audio/sfx/octopus_sound.wav"),
    (Sfx::OctopusShoot,     "audio/sfx/octopus_shoot.wav"),
    (Sfx::OctopusRush,      "audio/sfx/octopus_rush.wav"),
    (Sfx::OctopusDie,       "audio/sfx/octopus_die.wav"),
    (Sfx::MineBeep,         "audio/sfx/t_1.ogg"),
    (Sfx::MineExplode,      "audio/sfx/bomb.ogg"),
    (Sfx::BossStart,        "audio/sfx/boss_start.ogg"),
    (Sfx::BossStart2,       "audio/sfx/boss_start_2.ogg"),
    (Sfx::BossExplosion,    "audio/sfx/boss_explosion.ogg"),
    (Sfx::BossExplosion2,   "audio/sfx/boss_explosion_2.ogg"),
    (Sfx::GreenUfoSpawn,    "audio/sfx/green_ufo.ogg"),
    (Sfx::GreenUfoDeath,    "audio/sfx/green_ufo_death.ogg"),
    (Sfx::GatlingShoot,     "audio/sfx/gatling_shoot.ogg"),
    (Sfx::MothershipAlarm,  "audio/sfx/mothership_alarm.ogg"),
    // UI
    (Sfx::UiCountdownBeep,  "audio/sfx/t_1.ogg"),
    (Sfx::UiCountdownReady, "audio/sfx/t_ready.ogg"),
    (Sfx::UiCountdownGo,    "audio/sfx/t_go.wav"),
    (Sfx::UiPause,          "audio/sfx/pause.ogg"),
    // Cinématiques
    (Sfx::PlanetLanding,    "audio/sfx/landing.ogg"),
    (Sfx::ShipArrival,      "audio/sfx/landing.ogg"),
];

#[derive(Resource, Default)]
pub struct SfxLibrary {
    handles: HashMap<Sfx, Handle<AudioSource>>,
}

impl SfxLibrary {
    /// Retourne le `Handle` du SFX. Panique si la variante n'a pas été
    /// déclarée dans `SFX_PATHS` — c'est un bug de programmeur, on veut
    /// qu'il pète au démarrage, pas qu'il joue rien silencieusement.
    pub fn get(&self, sfx: Sfx) -> Handle<AudioSource> {
        self.handles
            .get(&sfx)
            .cloned()
            .unwrap_or_else(|| panic!("SFX {sfx:?} absent de SFX_PATHS"))
    }
}

fn preload_sfx(asset_server: Res<AssetServer>, mut library: ResMut<SfxLibrary>) {
    for (sfx, path) in SFX_PATHS {
        library.handles.insert(*sfx, asset_server.load(*path));
    }
}

/// Helper `SystemParam` pour jouer un SFX en une ligne dans n'importe quel
/// système. Retourne `EntityCommands` pour chaîner `.insert(marker)` quand
/// c'est utile.
#[derive(SystemParam)]
pub struct SfxPlayer<'w, 's> {
    commands: Commands<'w, 's>,
    library: Res<'w, SfxLibrary>,
}

impl<'w, 's> SfxPlayer<'w, 's> {
    /// One-shot avec auto-despawn. Cas par défaut.
    pub fn play(&mut self, sfx: Sfx) -> EntityCommands<'_> {
        self.commands.spawn((
            AudioPlayer::new(self.library.get(sfx)),
            PlaybackSettings::DESPAWN,
        ))
    }

    /// One-shot avec volume custom (multiplié au master). Auto-despawn.
    pub fn play_at(&mut self, sfx: Sfx, volume: f32) -> EntityCommands<'_> {
        self.commands.spawn((
            AudioPlayer::new(self.library.get(sfx)),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(volume)),
        ))
    }

    /// One-shot SANS auto-despawn (entité survit après lecture). Utile quand
    /// le son doit survivre une transition d'état (ex: `you_died.ogg` qui
    /// continue à jouer pendant qu'on passe à GameOver).
    pub fn play_once(&mut self, sfx: Sfx) -> EntityCommands<'_> {
        self.commands.spawn((
            AudioPlayer::new(self.library.get(sfx)),
            PlaybackSettings::ONCE,
        ))
    }
}

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SfxLibrary>()
            .add_systems(Startup, preload_sfx);
    }
}
