!!!! A RELIRE ET COMPLETE, MODIFIE, REDIGE PAR CLAUDE !!!!

# Game Design Document — Twin Stick Roguelike
**Version 0.1 — Draft**

---

## Table of Contents

1. [Overview](#overview)
2. [Core Pillars](#core-pillars)
3. [Structure of a Run](#structure-of-a-run)
4. [Boss System](#boss-system)
5. [Deckbuilding System](#deckbuilding-system)
6. [System Integration — How They Work Together](#system-integration)
7. [Card Taxonomy](#card-taxonomy)
8. [Progression & Balancing Notes](#progression--balancing-notes)

---

## Overview

A twin-stick roguelike shooter with 4 levels, each featuring a unique boss. Runs are non-linear: the player chooses the order in which they tackle the 4 bosses. Each boss drops a unique powerful reward card upon defeat. A deckbuilding system governs the player's abilities throughout the run, evolving through enemy encounters and level-up choices.

The core design challenge — and its solution — is making these two systems speak the **same language**, so that strategic boss ordering and emergent deckbuilding reinforce each other rather than compete.

---

## Core Pillars

- **Agency** — The player always feels in control of their build direction.
- **Strategic depth** — Boss order is a meaningful decision, not an afterthought.
- **Emergence** — No two runs feel the same due to card variance and boss sequencing.
- **Readability** — Systems are layered but individually learnable.

---

## Structure of a Run

A full run consists of:

1. **4 levels**, each containing multiple enemy encounters and ending with a boss fight.
2. The player chooses which level to enter next from a **world map / node selection screen**.
3. Each level is composed of a series of **encounters** (combat rooms).
4. After completing each encounter, the player is offered a **card draft** (choose 1 of 3 cards to add to their deck).
5. At the end of each level, the **boss fight** takes place.
6. Defeating a boss rewards the player with that boss's **unique Boss Card**.

---

## Boss System

### Concept

Inspired by the Mega Man formula: each boss has a unique identity, and defeating one gives the player an advantage against others. In this game, that advantage is expressed entirely through the card system.

### The 4 Bosses (placeholder names)

| Boss | Thematic Identity | Boss Card Reward |
|---|---|---|
| **Ignis** | Fire / Burn | *Pyroclasm* — AOE burst, applies Burn |
| **Glacius** | Ice / Slow | *Permafrost* — Freezes all enemies on screen |
| **Voltex** | Lightning / Chain | *Arc Surge* — Chains damage between enemies |
| **Noctis** | Shadow / Stealth | *Eclipse* — Player becomes invulnerable for 2s, resets cooldowns |

### Boss Weaknesses

Each boss is **weak to the Boss Card** of another specific boss. This creates a directed weakness graph:

```
Ignis  →  weak to  Glacius (Permafrost)
Glacius →  weak to  Voltex  (Arc Surge)
Voltex  →  weak to  Noctis   (Eclipse)
Noctis  →  weak to  Ignis    (Pyroclasm)
```

When a Boss Card is used against its corresponding weak boss, it deals **bonus damage** and triggers an additional **contextual effect** (e.g. Permafrost against Ignis extinguishes his flame shield). This rewards players who plan their boss order in advance.

---

## Deckbuilding System

### The Deck

The player starts each run with a small **starter deck** of 6–8 basic cards. The deck grows through :

- **Level-up drafts** — Killing enemies grants XP. On level-up, the player draws a randomized hand of 3 cards from the **global card pool** and picks one to add permanently to their deck.
- **Encounter drafts** — After each completed encounter room, the player picks 1 of 3 offered cards to add to their deck.

### Playing Cards — The Mana System

Cards are played during combat at each level-up.

- **Level-up drafts** — Killing enemies grants XP. On level-up, the player draws a randomized hand of 6 cards from the **deck** and picks some to play.
- The player has a **mana pool of 3**.
- Cards have a **mana cost of 1, 2, or 3**.
- Cards are drawn from the deck into a **hand** that appears at level-up.
- Played cards go to a **discard pile** and cycle back into the deck when it is exhausted.

This is intentionally inspired by Slay the Spire's economy, adapted for real-time twin-stick play.

### Card States

| State | Description |
|---|---|
| **In Deck** | Available to be drawn |
| **In Hand** | Currently accessible to the player |
| **Active** | Currently in effect (for persistent cards) |
| **Discarded** | Used, waiting to cycle back |

---

## System Integration

### The Core Principle

> **Boss order is a deckbuilding decision, not a separate decision.**

Rather than having boss rewards be a parallel system (a passive ability, a stat boost, etc.), **boss rewards are cards**. This means every choice the player makes — drafting from encounters, choosing a level-up card, and deciding which boss to fight next — operates within the same system and the same vocabulary.

### How It Works in Practice

When the player defeats a boss, the **Boss Card** is immediately added to their deck. It is a powerful, high-cost card (cost: 3) with a unique effect unavailable anywhere else in the card pool.

This creates the following chain of decisions:

1. **Before choosing a level**, the player evaluates their current deck. What does it lack? What synergies are emerging?
2. **Boss cards are known in advance** (visible on the world map). The player can factor the incoming Boss Card into their deck direction.
3. A player building a Burn/Fire deck will naturally want Ignis's *Pyroclasm* card — but they may want to fight Glacius *first* to get *Permafrost*, since it counters Ignis and makes that fight easier.
4. This creates a **genuine tension** between "what card do I want next?" and "what card makes the next fight easier?" — both answered through the same system.

### Boss-Gated Cards

In addition to Boss Cards themselves, certain **regular cards in the pool are locked** until a specific boss is defeated. These are mid-to-high power cards that become available in the encounter and level-up draft pools after the relevant boss dies.

Example:

- Defeating **Ignis** unlocks the *Ember Chain* and *Molten Core* cards in the pool.
- Defeating **Glacius** unlocks *Cryo Burst* and *Frost Nova*.

This means boss order also shapes **what you can draft**, not just what you receive directly. A player who fights Ignis first opens up a fire-synergy deck path for the rest of the run.

### Summary of Integration Points

| Decision | Deckbuilding Impact | Boss Strategy Impact |
|---|---|---|
| Which boss to fight next | Determines which Boss Card you receive and which card pool unlocks | Determines which boss weakness you'll have available |
| Encounter card drafts | Grows and shapes your deck | Prepares you (or not) for the next boss's mechanics |
| Level-up card selection | Deepens synergies | Can compensate for a missing boss weakness |
| Playing a Boss Card mid-run | Powerful in any context | Decisive against the matching weak boss |

---

## Card Taxonomy

Cards are organized into **types** and **tags**.

### Types

| Type | Description |
|---|---|
| **Weapon** | Modifies or adds a firing mode (e.g. spread shot, piercing round) |
| **Ability** | Active skill with a cooldown or mana cost (e.g. dash, shield) |
| **Passive** | Persistent effect active while the card is in play |
| **Reaction** | Triggers automatically on a condition (e.g. on taking damage) |
| **Boss Card** | Unique, high-power card obtained only from boss defeat |

### Tags (Elemental / Thematic)

Tags are used for synergy interactions between cards.

- `[Fire]` — Burn damage over time, damage amplification
- `[Ice]` — Slow, freeze, fragility (frozen enemies take bonus damage)
- `[Lightning]` — Chain effects, stun, mana refund on kill
- `[Shadow]` — Invulnerability windows, cooldown reset, clone effects
- `[Neutral]` — No elemental tag, fits any build

A card can have multiple tags. Certain cards have **synergy bonuses** when played alongside cards of the same tag (e.g. *Ember Chain* deals double damage if *Pyroclasm* is in your hand).

---

## Progression & Balancing Notes

### Deck Size

- Starter deck: 8 cards
- Recommended maximum: 20–24 cards (beyond this, consistency suffers)
- Players should be able to **remove cards** at specific rest nodes to keep the deck tight.

### Boss Card Balance

Boss Cards are intentionally **overpowered** against their matched weak boss, and merely **strong** in general use. This preserves the Mega Man fantasy without making the cards useless if the player uses them out of order.

### Difficulty Scaling

- Fighting a boss **without** its counter card should be difficult but winnable.
- Fighting a boss **with** its counter card should feel decisive and satisfying, not trivial.
- The sweet spot: a well-piloted deck without the counter should succeed ~50% of the time; with the counter, ~80%.

### Run Variance

Because the card pool is randomized per draft, two runs with the same boss order can feel completely different. The boss order sets the **skeleton** of the run; the card drafts fill in the **flesh**. This is the intended design — strategy and luck in healthy proportion.

---

*End of Document — v0.1*
