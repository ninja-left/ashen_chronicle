# The Ashen Chronicle

## Roadmap

## Phase 0: Project foundation

Goal: create the bare project structure and lock the core design.

Done when:
- the Rust project exists
- the game name is finalized
- the design plan and roadmap are written
- the codebase has a clean folder structure

Status: complete.

## Phase 1: Engine skeleton

Goal: prove the game can exist as a stable simulation.

Work items:
- save/load
- world state model
- entity IDs
- basic character creation
- text UI
- simple turn/state loop

Status: complete.

Completed so far:
- world state model exists
- stable entity IDs are used for world content and history entries
- basic character creation is wired into the game loop
- text UI menus and prompts exist
- save/load is implemented through a versioned JSON save file
- a simple travel/history/death loop is playable
- the menu uses a single Back label in submenus instead of Cancel/Exit duplication
- the recovery action is meditate, and it heals plus saves when the player is safe
- a first threat state exists so meditation is blocked in dangerous places

Done when:
- a new game can start
- a world can be saved and loaded
- a character can be created and stored

## Phase 2: Basic world loop

Goal: make the game playable.

Work items:
- procedural regions
- locations
- travel
- encounters
- inventory
- simple combat
- death handling
- new world or inherited world choice

Status: complete.

Completed so far:
- the player can move through a generated world
- the player can die
- the death screen offers world reset or world inheritance
- safe meditation and saving are tied to world danger
- a basic threat interaction exists
- threat resolution now has a real combat loop
- defeating a threat can clear danger and leave a trophy item behind
- item pickups now notify the player directly
- major action results now pause on screen so the text does not get skipped

Done when:
- the player can move through a generated world
- the player can die
- the death screen offers world reset or world inheritance

## Phase 3: World consequence systems

Goal: make the world remember what happened.

Work items:
- corpses
- item drops
- persistent world changes
- faction reputation
- history log
- NPC memory basics
- simple quest system

Status: complete.

Completed so far:
- corpses are created when a character dies
- items drop into the corpse inventory instead of disappearing
- corpse remains persist in the world after inheritance
- locations show visible remains
- the player can search remains and recover items
- history records death, looting, and aftermath events
- danger can be cleared by defeating a threat, changing the world state
- faction reputation is tracked and changes when the player completes the shrine quest
- NPCs remember important events and react differently on later visits
- the first simple quest is active and rewards the player

Done when:
- the old character leaves visible traces in the world
- a new character can encounter those traces
- the world state meaningfully changes across runs

## Phase 4: Content expansion

Goal: make the game feel rich instead of barebones.

Work items:
- more items
- more enemies
- more locations
- more events
- more factions
- more quest chains
- better descriptive writing tools

Done when:
- the game has enough variety for repeated runs without feeling empty

## Phase 5: Data loading and mod foundation

Goal: let content grow without turning code into a mess.

Work items:
- external content loading
- validation
- override rules
- dependency tracking
- error reporting
- stable content IDs

Done when:
- new content can be added mostly through data files
- broken content is caught before it corrupts the game

## Phase 6: Mod support

Goal: make the game extensible by design.

Work items:
- add mod folders or packages
- allow content replacement and extension
- expose safe hooks for events and quests
- document mod rules

Done when:
- a mod can add or override content without editing engine code

## Phase 7: Optional visuals

Goal: support images without changing the core structure.

Work items:
- portraits
- item images
- location art
- map or scene panels

Done when:
- visuals enhance the text instead of replacing it

## Phase 8: Polish and balance

Goal: turn the system into a real game.

Work items:
- balance passes
- content cleanup
- UI cleanup
- bug fixing
- save compatibility checks
- clearer feedback and logs

Done when:
- the game is stable enough for long play sessions
- progression, death, and inheritance all feel fair

## Tracking rule

Use this roadmap as the main progress checklist. If a feature is not helping one of the current milestone goals, it should not become a priority yet.
