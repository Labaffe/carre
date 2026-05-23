# Système d'items

[src/item/item.rs](src/item/item.rs) gère les items droppés par les
entités mortes. Architecture event-driven.

## Flow

1. Une entité avec `DropTable` meurt → `detect_death` (ou un système
   custom comme `kamikaze_boom_system`) émet un `DropEvent`
2. `process_drop_events` spawne les items (sprite animé OU statique,
   descente verticale) selon les probabilités de la table
3. Le joueur touche un item → effet appliqué + son + despawn

## Types d'items

| Item | Effet | Sprite |
|------|-------|--------|
| `Bomb` | +1 bombe au compteur `PlayerBombs` | `images/bomb/` (5 frames animées) |
| `BonusScore` | +50 au score | `images/bonus_score/` (5 frames animées) |
| `Armor` | +1 armure (cap `PLAYER_MAX_ARMOR = 3`). Si déjà au cap, l'item est consommé sans effet. | `images/armor.png` (1 frame statique, taille réduite via `ItemType::sprite_size()`) |

Sons : `Sfx::ItemAppear` (drop) et `Sfx::ItemPickup` (ramassage).
Voir [docs/audio.md](audio.md).

## Bombes

LShift consomme 1 bombe :
- Flash blanc plein écran (fade out 0.4s, composant `BombScreenFlash`)
- `Sfx::PlayerBomb` (volume 3.0)
- Émet `DamageEvent` pour tous les `Asteroid` (`BOMB_DAMAGE_ASTEROID = 999`)
  et tous les `Enemy` non-kamikaze (`BOMB_DAMAGE_ENEMY = 50`)
- Despawn direct des kamikazes (bypass damage event pour éviter un AOE
  de mort qui touche le joueur)
- Despawn direct de tous les projectiles ennemis et toutes les AOE actives
  (effet "panique" plein écran)

### UI bombes

Position : `top: 148, left: 20` (sous l'armure).
- 1 icône `bomb/frame000.png` (64px). **Grisée** (`HUD_INACTIVE_COLOR`) quand `count == 0`, blanche sinon.
- Texte `x N` à droite, même couleur que l'icône.
- Hint `[LSHIFT]` clignote dessous (visible uniquement quand `count > 0`).

## Armure (player)

Composant `Armor { current: u32, max: u32 }` sur le joueur, max =
`PLAYER_MAX_ARMOR = 3`. Voir [src/player/player.rs](src/player/player.rs).

### Pipeline

Quand `apply_damage` reçoit un `DamageEvent` pour le joueur :
1. Si `Armor.current > 0` : consomme `min(armor, damage)` points d'armure.
2. Reste éventuel → `Health::take_damage`.
3. `HitEvent` est trigger si au moins 1 point a été absorbé (armure OU
   vie) → les FX (flash, son `PlayerHurt`, `Invincible`) jouent dans les
   deux cas.

### UI armure

Position : `top: 96, left: 20` (sous les vies).
- 3 slots d'icônes `armor.png` (40px chacune).
- Hidden quand le slot n'est pas possédé (contrairement aux vies, on
  n'affiche pas un placeholder grisé).
- Si `current = 2`, on voit 2 icônes alignées à gauche.

## UI vies

Position : `top: 20, left: 20`.
- 3 icônes `health.png` (64px).
- **Toutes toujours visibles**, mais grisées (`HUD_INACTIVE_COLOR`) pour
  les vies perdues. Pattern "icônes éteintes" → on voit le total et ce
  qui manque d'un coup.

Layout vertical complet (left: 20) :
- `top: 20` → Vies (3 × 64px)
- `top: 96` → Armure (0-3 × 40px)
- `top: 148` → Bombes (icône 64 + "x N" + hint LSHIFT)
- `top: 224` → Arme (case 48 + nom à droite)

Couleur "inactive" partagée : `crate::player::player::HUD_INACTIVE_COLOR`.

## Drop tables

Définies en `static` à côté de chaque ennemi qui drop :

- **Asteroid** ([asteroid.rs](src/enemy/asteroid.rs)) : `[(Bomb, 5%), (BonusScore, 10%), (Armor, 3%)]`
- **Kamikaze** ([kamikaze.rs](src/enemy/kamikaze.rs)) : `[(Bomb, 10%), (BonusScore, 15%), (Armor, 8%)]`
- **GreenUFO** ([green_ufo.rs](src/enemy/green_ufo.rs)) : `[(Bomb, 10%), (BonusScore, 15%), (Armor, 8%)]`
- **Turret** ([turret.rs](src/enemy/turret.rs)) : `[(BonusScore, 20%), (Armor, 5%)]`

L'octopus / boss / mine ne droppent rien (pas de `DropTable` attaché au spawn).

Chaque entrée est testée indépendamment (probas ne se somment pas).

## Spawn d'items

`process_drop_events` lit `DropEvent`, pour chaque entrée :
1. Roll `fastrand::f32()` contre la proba
2. Si succès, spawn une entité avec :
   - Sprite (frame initial de l'animation)
   - `Transform::from_translation(event.position)`
   - `Droppable { item_type }`
   - `ItemAnim { frames, index, timer }` (anim en boucle)
   - `collider(Circle(ITEM_PICKUP_RADIUS), layers::ITEM, layers::PLAYER)`
3. Joue `Sfx::ItemAppear` (volume 3.0)

Taille du sprite : par défaut `ITEM_SPRITE_SIZE = 72.0`, override par
variante via `ItemType::sprite_size()` (armor = 44.0).

## Pickup

`item_pickup_on_overlap` lit `OverlapEvent` (couples PLAYER ↔ ITEM) :
1. `match droppable.item_type`:
   - `Bomb` : `bombs.count += 1`
   - `BonusScore` : `score.add(BONUS_SCORE_VALUE = 50)`
   - `Armor` : `armor.current = (current + 1).min(max)` (cap à 3)
2. Joue `Sfx::ItemPickup` (volume 3.0)
3. Despawn l'item

## Cleanup hors écran

`cleanup_offscreen_droppables` despawn les items qui tombent sous le bord
inférieur de la fenêtre + 50px de marge.
