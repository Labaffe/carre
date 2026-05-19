# Système d'items

[src/item/item.rs](src/item/item.rs) gère les items droppés par les entités mortes. Architecture event-driven.

## Flow

1. Une entité avec `DropTable` meurt → `DropEvent` émis
2. `process_drop_events` spawne les items (sprite animé, descente)
3. Le joueur touche un item → effet appliqué + son + despawn

## Types d'items

| Item | Effet | Animation |
|------|-------|-----------|
| `Bomb` | +1 bombe au compteur | `images/bomb/` |
| `BonusScore` | +50 au score | `images/bonus_score/` |

Sons : `Sfx::ItemAppear` (drop) et `Sfx::ItemPickup` (ramassage). Voir [docs/audio.md](docs/audio.md).

## Bombes

Le joueur accumule des bombes (UI sous les vies). Appuyer sur Espace consomme 1 bombe :
- Flash blanc plein écran (fade out 0.4s)
- `Sfx::PlayerBomb`
- Dégâts à tous les astéroïdes (`BOMB_DAMAGE_ASTEROID = 999`) et ennemis actifs (`BOMB_DAMAGE_ENEMY = 50`)
- Texte `[ESPACE]` clignote sous les icônes de bombes quand compteur > 0

## Drop tables

- Astéroïdes : `[(Bomb, 5%), (BonusScore, 10%)]`
- GreenUFO : `[(Bomb, 10%), (BonusScore, 15%)]`
- Kamikaze : `[(Bomb, 10%), (BonusScore, 15%)]`

Chaque entrée est testée indépendamment.
