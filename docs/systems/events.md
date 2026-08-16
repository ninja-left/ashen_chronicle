# Event System

The event system provides reusable, data-driven narrative and gameplay events.

## Runtime

Events are defined by stable IDs and support triggers, weighted selection, chance gates, conditions, effects, and cooldowns. Travel events were migrated from hardcoded logic into the campaign content layer.

Campaign content is cached in runtime state so event processing does not reload the content pack for every trigger.

## Conditions

The runtime supports conditions based on prior events, faction reputation ranges, required inventory items, active character conditions, and other validated content references. Invalid event definitions are rejected with warnings while valid content continues loading.

Prior-event references allow events to react to persistent history without coupling the event system to a specific gameplay flow.

## Persistence

Event cooldowns are part of persistent world state and survive save/load and world inheritance. Executed events are recorded in structured history with event ID, location, and outcome data.

## Content validation

Validation covers duplicate IDs, invalid triggers, weights, chance ranges, effects, cooldown-related data, condition references, faction references, and prior-event references. Mod content is validated independently and may safely refer to compatible base content.

## Design direction

Events should remain reusable and data-driven. New event behavior should normally be expressed through content definitions and the existing runtime rather than by adding one-off event branches to gameplay code.
