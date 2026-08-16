# v0.28.0 Architecture Milestone

- extracted gameplay action dispatch from `game/runtime.rs` into `game/dispatcher.rs`
- kept the runtime loop focused on lifecycle, presentation, menu selection, and turn progression
- preserved existing action, combat, screen, world, save, and quit behavior
- bumped the project version to v0.28.0

Note: this file is a temporary milestone record. The canonical `ROADMAP.md` entry should be reconciled in the next documentation maintenance pass.
