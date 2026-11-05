# A04: accepted meaning of a life-saving arrow

Owner clarification: after a Medic's crossbow bolt heals a teammate, damage
that actually arrives immediately afterward (afterburn, flame damage, a
projectile, or another valid damage source) would have killed the teammate
without the arrow. This is a short, observed-damage counterfactual, not a
claim that the enemy would have chosen the same actions indefinitely.

The crossbow_heal event records healer, target, and amount; the SDK emits
user IDs. Generic player_healed events alone cannot identify an arrow.
Source: ValveSoftware/source-sdk-2013,
src/game/server/tf/tf_projectile_arrow.cpp, HealingArrowTouch.

Planned accounting:

1. Establish same-life, same-player pre-arrow health, effective healing,
   event order and post-arrow health. Retain demo/server tick separation.
2. Replay actual subsequent damage in order against a no-arrow health
   ledger over a calibrated short window. Use actual effective damage,
   not nominal weapon damage. All damage types qualify.
3. Count as a supported short-window save only if the no-arrow ledger
   reaches zero while the actual teammate remains alive at that moment.
   Record the decisive damage tick and health margins.
4. Account for intervening healing, regeneration, health pickups and
   protection changes. If raw intended healing/caps or effective damage
   cannot be reconciled, mark uncertain rather than over-credit the bolt.
   Other arrow/medigun heals must not be double-counted with duplicate
   event representations.
5. Stop/reject attribution across respawn, round/identity boundaries,
   missing temporal evidence or unobserved health changes. Do not infer
   survival solely from absence of a death event.

Examples for later acceptance fixtures:

| Before | Effective arrow heal | Observed next damage | Actual health after damage | No-arrow ledger | Expected |
| ---: | ---: | ---: | ---: | ---: | --- |
| 40 | 75 | 60 | 55 | -20 | Supported save if continuity verified |
| 6 | 75 | 4 + 4 afterburn | 73 | -2 | Supported save at second damage tick |
| 100 | 30 | 20 | 110 | 80 | No save |
| 40 | 75 | 60, but teammate also gets another heal | Depends | Must replay other heal/cap | Uncertain until reconciled |
| 40 | 75 | Actual teammate dies anyway | 0 | <= 0 | Not a surviving save at that death |

The exact immediate-window duration and candidate/scoring activation require
a concrete calibration proposal. Existing UI, scores, tags and candidate
generation remain unchanged. This document records the owner's clarified
requirement; it is not a claim that A04 is implemented.
