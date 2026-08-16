# The Ashen Chronicle

## Development Plan

The game is a persistent-world, text-first RPG built around permadeath, world inheritance, data-driven content, and modular architecture.

## Development priorities

1. Preserve the persistent-world model: a character can die without erasing the world.
2. Keep gameplay systems data-driven so content can expand without engine rewrites.
3. Keep system boundaries explicit and avoid coupling unrelated responsibilities.
4. Preserve save compatibility and world-state continuity as systems evolve.
5. Build and test systems incrementally rather than introducing large coupled features.

## Current architecture direction

The codebase is being decomposed by responsibility. Runtime orchestration, gameplay actions, combat, screens, world/bootstrap logic, presentation, content loading, events, persistence, and UI should remain independently understandable.

See [`docs/architecture.md`](docs/architecture.md) for the architectural model and [`docs/systems/`](docs/systems/) for system-specific documentation.

## Development approach

Gameplay content should remain external to core logic where practical. Content must have stable IDs, schemas, validation, and explicit references. World mutations should go through clear ownership boundaries, and important state changes should be persisted and represented in world history where appropriate.

New systems should include focused tests for the behavior they introduce or modify. Refactors should preserve existing gameplay, save compatibility, and screen flow unless a change explicitly targets those areas.

## Mod/content progression

Mod support should evolve incrementally:

- Base campaign content loaded from structured data.
- External mod discovery and validation.
- Stable-ID-based replacement and extension.
- More advanced scripted hooks only after the data-driven foundation is stable.

## Long-term goals

The project should grow from the current text-first RPG into a deeper persistent-world simulation without turning into a collection of tightly coupled special cases. Procedural generation, quests, events, progression, memory, remnants, presentation, and mod support should continue to build on the same clear ownership and data-driven foundations.

## Working rules

- Every entity has a stable unique ID.
- References use IDs rather than display names.
- Systems own their state and expose explicit mutation paths.
- Content is validated before entering runtime state.
- Avoid splitting files purely to reduce line counts; split at responsibility boundaries.
- Preserve compatibility where practical, and add focused tests when behavior changes.

## Historical plan

The original development plan is preserved in [`docs/development-plan-history.md`](docs/development-plan-history.md).
