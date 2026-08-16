# UI System

The UI is a keyboard-driven ratatui terminal interface designed to remain usable across wide desktop terminals and narrow mobile portrait terminals.

## Screen architecture

The game uses dedicated screen flows for start, save selection, character creation, gameplay, quit, and death. Menu screens do not render the gameplay dashboard underneath them.

Start flow explicitly exposes `New Game`, `Load Game`, and `Quit`. Load is shown only when a compatible save exists. Quit and death flows remain non-destructive.

## Gameplay dashboard

The main dashboard presents state and current-turn results without accumulating stale output. Location atmosphere and scene art belong to the Location panel, while action outcomes are presented through a short-lived Result panel.

Combat exposes player and enemy health through ratatui `LineGauge` widgets. Redundant debug information and repeated state summaries were removed from the always-visible layout.

## Responsive behavior

Layout switches to compact rendering for narrow or tall terminals. Prompt overlays use available screen space efficiently, and longer option lists can scroll instead of overflowing small displays.

## Input

Interaction uses raw-mode keyboard input with arrows, Enter, Esc, and number shortcuts. Pause prompts use single-key confirmation. Prompts and confirmations are docked into a reserved bottom panel so they do not obscure the main dashboard.

## Visual content

ASCII portraits, location art, item illustrations, and screen-specific dark artwork are optional. Text-only fallback remains available when visual assets are missing.

The general UI style is monochrome except for contextual combat health fills.

## Design direction

Keep screen responsibilities separate from gameplay mechanics. New UI work should improve readability and responsive behavior without reintroducing persistent clutter or overlapping modal layouts.
