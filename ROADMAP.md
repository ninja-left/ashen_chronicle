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
- the first simple quest now requires a turn-in at Mira and no longer auto-completes at the shrine
- hidden quests stay out of the quest log until they are actually offered
- inherited worlds no longer misattribute old quest deeds to a new character
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

Status: complete.

Completed so far:
- the region expanded from 3 to 8 connected locations
- three distinct dangerous locations now have different enemy profiles
- additional enemy types and location-specific trophies were added
- the item pool now contains multiple trophies and faction rewards
- the faction roster expanded to three factions
- the NPC roster expanded to five named NPCs with persistent memories
- the quest system now supports multiple active quest chains using the same offer/turn-in rules
- location arrival scenes provide distinct atmospheric events and descriptions
- campaign bootstrap now backfills new content into older saved worlds instead of requiring a reset
- campaign content definitions are centralized in the bootstrap layer to prepare for the later data-loading phase

Done when:
- the game has enough variety for repeated runs without feeling empty

## v0.8.1 Quest-system maintenance

Completed:
- new characters no longer inherit the previous character's personal quest log
- completed quests are stored as persistent world deeds so later characters do not receive duplicate quests
- existing completed quest records are migrated into persistent world deeds during bootstrap/inheritance
- the main location description no longer prints a duplicate quest summary; Quest Log remains the dedicated quest view
- required quest items are consumed when a quest is successfully turned in
- fixed campaign quest bootstrapping so the shrine quest is not recreated on every load


## v0.8.2 Quest interaction and reputation maintenance

Completed:
- quest offering and turn-in now use an explicit Talk action with NPC selection
- same-location quest completion now works immediately after the threat is defeated
- NPCs no longer automatically offer or complete quests merely because the player entered a location
- new characters start with zero faction reputation while persistent faction memories remain
- quest completion grants +5 faction reputation and the associated faction reward contributes another +5 while held
- inherited reward items restore their +5 faction contribution when the new character recovers them from a previous life

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


## v0.9.0 Data-loading foundation

Completed:
- added a runtime-loaded campaign content file under `data/base_content.json`
- seeded the world from loaded content instead of hardcoded location bootstrap
- added validation for duplicate content IDs and broken content references
- moved NPC, faction, quest, atmosphere, and encounter definitions into the content pack
- added stable content IDs to the base campaign data
- kept existing save data compatible through defaulted quest fields

## Phase 6: Mod support

Goal: make the game extensible by design.

Work items:
- add mod folders or packages
- allow content replacement and extension
- expose safe hooks for events and quests
- document mod rules

Done when:
- a mod can add or override content without editing engine code


## v0.10.0 Mod loading foundation

Completed:
- added mod discovery under `data/mods`
- loaded mod content after the base content pack
- merged content by stable IDs and location keys so mods can replace or extend data safely
- kept broken mod files from crashing the game by reporting load warnings
- moved quest identity handling onto stable content IDs in the runtime paths

## Phase 7: Optional visuals

Goal: support images without changing the core structure.

Work items:
- portraits
- item images
- location art
- map or scene panels

Done when:
- visuals enhance the text instead of replacing it

## v0.11.0 Optional visuals foundation

Completed:
- added optional ASCII portraits for NPCs
- added optional location scene art
- added optional item illustrations for rewards and trophies
- kept visuals fully optional so the game still works as text-only when assets are missing


## Phase 8: Polish and balance

Status: complete.

Completed so far:
- location art and scene presentation are now integrated into the play loop
- combat, questing, meditation, inventory, and death handling have stable player-facing flows
- the remaining work now centers on presentation cleanup and terminal UX instead of core gameplay scaffolding

Done when:
- the core systems feel cohesive and readable

## Phase 9: Terminal UI foundation

Goal: replace line-by-line terminal output with a persistent screen layout.

Work items:
- alternate-screen terminal wrapper
- structured dashboard for world, character, and location state
- event log panel
- modal prompts and numeric selection menus
- preserve existing text scenes inside the new screen shell

Status: complete.

Completed so far:
- the game now boots inside an alternate-screen terminal session
- the main view renders as a persistent dashboard instead of a pure scrollback stream
- story output is collected into an on-screen log panel
- prompts and choice menus now render through the shared UI layer
- the version has been bumped to v0.15.0 to mark the UI foundation change

Done when:
- the game no longer depends on raw scrollback for normal play

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

## v0.12.0 Polish and balance pass

Completed:
- simplified the main status screen to remove repeated world-mode and history-count clutter
- removed repeated location and inventory ASCII art from the always-visible screens while retaining contextual visuals during interaction and item acquisition
- tightened combat output so attacks and damage resolve in one readable sequence instead of pausing after every hit
- added explicit combat victory and quest reward summaries
- added clearer save-load validation warnings for broken runtime references and inconsistent completed quest deeds
- adjusted dangerous encounters to reduce early-game damage spikes while preserving different threat durability
- reduced repetitive travel/location presentation by keeping the recurring status display focused on actionable information

## v0.12.1 Quit-screen polish

Completed:
- replaced the plain quit confirmation with a randomized atmospheric exit screen
- added four original dark farewell variants with contextual ASCII art
- kept quitting non-destructive: `N`, `No`, or Enter returns to the game without changing state
- bumped the project version to 0.12.1

## v0.13.0 Gameplay polish and balance completion

Completed:
- added an atmospheric end-of-life screen with a concise summary of death location, remembered deeds, faction standing, and items left on the corpse
- made previous-life traces discoverable through corpse recovery and NPC memories without restoring the dead character's quest log
- added a small set of randomized, non-quest travel events to make repeated journeys less predictable without changing important quest outcomes
- added explicit corpse/legacy feedback so inherited equipment is distinguished from information that must be rediscovered
- retained the existing faction reputation split and prevented new characters from inheriting old reputation

## Tracking rule

Use this roadmap as the main progress checklist. If a feature is not helping one of the current milestone goals, it should not become a priority yet.


## Phase 9: Character progression and living time

Goal: make each life develop over time while the world continues to age between lives.

Status: complete.

## v0.14.1 Character progression, NPC availability, and Time foundation

Completed:
- added Might, Insight, and Endurance attributes with level-based progression
- added experience gains from combat, quests, and discovery, with player-chosen level improvements
- added a character sheet for progression and condition visibility
- added Wounded, Exhausted, and Well-rested conditions with gameplay effects
- added a persistent world time cycle with named day/night portions, day count, and an east-to-west ASCII sun/moon track
- replaced the time track with a compact two-line celestial cycle and a clear east-to-west indicator line
- kept the sun and moon readable with plain Unicode symbols while hiding the internal time variable from player-facing text
- improved unavailable NPC feedback so it explains whether it is too early or too late and hints when to check again
- made travel, combat, searching, journaling, talking, and meditation advance hidden time portions
- made meditation duration player-selected and tied healing directly to time spent
- added time-sensitive NPC availability and travel atmosphere changes
- kept progression and conditions character-specific while world time persists through inheritance
- added v1 save migration for the new progression/time fields
- added small time-sensitive travel variation to exercise the new systems

## v0.14.2 Location scene art presentation

Completed:
- rendered optional location scene art during arrival scenes so the player sees the place before the descriptive text
- kept scene art fully optional and text-only fallback intact when no art is defined
- reused the existing atmosphere and NPC scene flow so visual content layers cleanly onto the text systems

## Phase 10: Keyboard interaction pass

Goal: replace the remaining typed-number interaction flow with a keyboard-first terminal UI.

Work items:
- arrow-key and Enter navigation for menus
- raw-mode text entry for prompts
- clearer modal feedback for pauses and confirmations
- keep scene art and narrative logs inside the same screen shell

Status: complete.

## v0.16.0 Keyboard-driven interaction pass

Completed:
- replaced the blocking number-entry menus with keyboard-driven selection using arrows, Enter, Esc, and number shortcuts
- added raw-mode text input so prompts work directly inside the TUI without falling back to scrollback interaction
- changed pause handling to a single-key confirmation instead of Enter-only input
- updated the on-screen control hint to match the new keyboard flow
- bumped the project version to 0.16.0

## Phase 11: Responsive ratatui renderer

Goal: make the terminal UI survive narrow screens and modernize the drawing layer.

Work items:
- ratatui-based rendering backend
- compact layout for vertical/mobile terminals
- wide layout for desktop terminals
- modal prompt overlay and centered popups
- resize-safe panel rendering

Status: complete.

Completed so far:
- the provisional hand-rolled output shell was replaced with a ratatui-backed renderer
- the UI now switches between compact and wide layouts based on terminal width and height
- prompts and menus render in centered overlays instead of relying on raw scrollback
- the existing gameplay screens keep working inside the new layout shell

## v0.17.0 Responsive ratatui renderer

Completed:
- migrated the terminal renderer to ratatui
- added width/height-aware compact rendering for narrow vertical screens
- kept the existing gameplay flows working inside the new screen shell
- bumped the project version to 0.17.0


## v0.17.1 Mobile portrait UI cleanup

Completed:
- expanded compact-mode detection so tall, narrow terminals no longer get forced into the desktop split layout
- made prompt overlays use more of the available screen space on compact terminals
- added scrolling window behavior for menu overlays so longer option lists stay readable on smaller screens
- bumped the project version to 0.17.1

## v0.17.2 Monochrome prompt cleanup

Completed:
- switched the UI styling to monochrome gray/white borders and highlights
- moved the pause prompt out of the center overlay so result text stays visible
- reduced the choice popup footprint so it does not bury the rest of the screen as aggressively
- bumped the project version to 0.17.2
## v0.17.3 Docked prompt layout cleanup

Completed:
- moved prompt and confirmation dialogs into a reserved bottom panel instead of drawing them over the rest of the UI
- kept the main dashboard visible while choices, pause prompts, and quit confirmations are active
- tightened the prompt layout so messages and results stay readable behind the prompt flow
- bumped the project version to 0.17.3


## v0.17.4 Turn-based result cleanup

Completed:
- removed the duplicate top-of-screen game-state summary so the header no longer repeats the Status panel
- changed the Messages panel into a short-lived Result panel that is cleared when the player starts a new choice
- kept action outcomes visible only for the current turn instead of letting old text pile up indefinitely
- tightened the header layout to give landscape screens more usable space
- bumped the project version to 0.17.4

## v0.17.5 Main-screen cleanup

Completed:
- removed the empty top header box instead of leaving it as a hollow frame
- removed debug-style location exits and people listings from the main dashboard
- removed the character name and faction reputation from the main dashboard
- moved faction reputation into the character sheet
- moved location arrival art and atmosphere into the Location panel instead of the Result panel
- fixed the quit confirmation prompt so it no longer prints a stray `>` on separate lines
- bumped the project version to 0.17.5



## v0.17.6 UI polish and health gauges

Completed:
- replaced plain-text player HP rendering with a ratatui `LineGauge`
- added an enemy health `LineGauge` during combat
- used blood/dark-red player health and dark-purple enemy health fills while retaining the monochrome UI elsewhere
- kept combat health in the dashboard state so it updates while combat choices are rendered
- normalized panel rendering and collapsed border overlap across compact and wide layouts
- removed redundant combat HP text output and simplified the result display
- bumped the project version to 0.17.6

## Phase 12: Data-driven event system

Goal: move world events out of hardcoded gameplay branches and establish reusable trigger, condition, selection, effect, and cooldown infrastructure.

Work items:
- event content definitions with stable IDs and trigger names
- conditional event filtering for time, location, and danger state
- weighted and chance-based event selection
- persistent event cooldown state in world saves
- reusable event effects for narrative output and future gameplay effects
- validation and focused unit tests for event behavior
- migrate existing travel events from `game.rs` into base content

Status: complete.

## v0.18.0 Event system foundation

Completed:
- added a reusable event runtime in `src/events.rs`
- added stable event IDs, triggers, weights, chance gates, conditions, effects, and cooldowns to campaign content
- migrated the four existing travel events into `data/base_content.json`
- persisted event cooldowns as part of the world state with serde defaults for older saves
- added content validation for event IDs, triggers, weights, effects, chance ranges, and condition references
- added unit tests covering event conditions and cooldown eligibility
- removed the old hardcoded random travel-event branch from `game.rs`
- bumped the project version to 0.18.0

## v0.18.1 Event validation and persistence test coverage

Completed:
- reject invalid individual event definitions instead of loading them into the runtime
- emit warnings identifying each rejected event and the exact validation reasons while continuing to load valid content
- reject duplicate event IDs without overwriting previously accepted events
- added save/load coverage for persistent event cooldowns
- added world-inheritance coverage confirming event cooldown state persists with the inherited world
- added content filtering tests for invalid events and duplicate event IDs
- bumped the project version to 0.18.1
