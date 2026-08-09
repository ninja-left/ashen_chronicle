# The Ashen Chronicle

A text-driven dark fantasy RPG built in Rust, focused on exploration, consequence, character death, and a world that remembers what happened.

The game is inspired by the atmosphere and themes of dark fantasy: dangerous roads, forgotten places, strange characters, difficult choices, and a world that does not simply reset when a character dies.

## Features

- Procedurally generated world
- Character creation
- Exploration and travel
- Threats and encounters
- Turn-based combat
- World reset and inheritance options on death
- Persistent world changes
- History/event tracking
- NPC memory, reactions, and Faction reputation
- Quest system
- Base content loading and mod support foundation
- Text-based user interface

## Core Concept

Death is not simply a game over.

When a character dies, their actions can leave traces behind. Their remains, deeds, consequences, and changes to the world can persist.

A later character may inherit that world and encounter what the previous character left behind.

The goal is to make the world feel persistent rather than disposable.

## Project Structure
```
ashen_chronicle/
├── data/
│   ├── base_content.json
│   └── mods/
├── src/
│   ├── content.rs
│   ├── game.rs
│   ├── main.rs
│   ├── model.rs
│   ├── persistence.rs
│   └── ui.rs
├── Cargo.toml
├── Cargo.lock
├── DEVELOPMENT_PLAN.md
├── ROADMAP.md
├── LICENSE
└── README.md
```

"game.rs" contains the game logic and gameplay flow.

"main.rs" is the application entry point.

"model.rs" contains the core game data structures and world model.

"persistence.rs" handles saving and loading the world.

"ui.rs" handles the text-based interface and player interaction.

"DEVELOPMENT_PLAN.md" contains the project's detailed development rules and design direction.

"ROADMAP.md" is the main progress tracker for development.

## Building

The project uses Rust and Cargo.

Build the project with:

```sh
cargo build -r
```

Run the game with:

```sh
cargo run -r
```

or Run the built release

```sh
./ashen_chronicle
```

Run the test suite with:

```sh
cargo test
```

## Design Philosophy

The Ashen Chronicle is being developed around several principles:

#### The world should remember.
Important actions should have consequences that can survive beyond a single character.

#### Death should matter.
Character death is part of the game's progression rather than merely a failure state.

#### Systems should interact.
Quests, factions, NPCs, locations, combat, inventory, history, and world state should gradually become interconnected rather than existing as isolated mechanics.

#### Content should eventually be data-driven.
As the game grows, adding content should require less modification of the underlying engine.

#### Text comes first.
The game is designed around its world, writing, atmosphere, and systems. Future visuals should enhance that foundation.

#### Keep the project maintainable.
Development should proceed incrementally, with the roadmap tracking completed milestones and semantic versioning tracking releases.

## Versioning

The project follows semantic versioning:

MAJOR.MINOR.PATCH

Development releases remain below "1.0.0" while the core systems and content are still being established.

## License

See "[LICENSE](./LICENSE)" for the project's license information.

---

The world remembers what you leave behind.
