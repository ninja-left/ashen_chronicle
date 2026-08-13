# Ashen Chronicle Development Rules

## Architecture
- Preserve the existing separation between model, game logic, persistence, UI, and content.
- Prefer extending existing systems over introducing parallel implementations.

## Persistence
- Existing save data must remain readable unless a migration is explicitly implemented.
- Do not silently discard persisted state.

## Content
- Keep game content data-driven where the existing architecture supports it.
- Avoid hardcoding content into gameplay logic unnecessarily.

## Rust
- Follow existing project conventions.
- Avoid unnecessary dependencies.
- Prefer clear, maintainable code over clever abstractions.

## Scope
- Do not modify unrelated systems as part of a feature.
