# Content System

The content system moves campaign definitions out of gameplay code and loads them as validated runtime data.

## Base content

The base campaign is stored in `data/base_content.json`. Stable content IDs identify locations, factions, NPCs, quests, encounters, atmosphere, and events. World creation seeds game state from the loaded definitions instead of relying on large hardcoded bootstrap tables.

## Modules

The content implementation was split into three responsibilities:

- `content/definitions.rs` contains data structures and validation rules.
- `content/loader.rs` handles base/mod discovery, loading, merging, event filtering, path discovery, and loader tests.
- `content/seeding.rs` translates validated content into the initial world state.

`content.rs` remains a compatibility-facing facade for the existing loading entry point.

## Mods

Mods are discovered under `data/mods` and loaded after the base content. Content is merged using stable IDs and location keys, allowing mods to replace or extend compatible definitions without duplicating the whole campaign.

Broken mod files produce load warnings instead of crashing the game. Mod validation is performed alongside the base validation rules and can reference compatible base content where the schema allows it.

## Validation

Content loading validates duplicate IDs, broken references, event definitions, faction references, reputation ranges, and other cross-content dependencies. Invalid definitions are rejected while valid content continues loading when possible.

## Optional assets

Content may reference optional ASCII portraits, location scene art, and item illustrations. Missing visual assets must not prevent the text-only game from running.

## Compatibility and runtime use

Campaign content is runtime data rather than save data. Saves keep persistent world state and rehydrate the current campaign definitions when loaded. The runtime caches loaded campaign content so repeated gameplay actions do not reload files unnecessarily.

## Design direction

Keep campaign data declarative and stable-ID based. Put reusable content in data files and keep game code responsible for generic behavior rather than embedding campaign-specific definitions in control flow.
