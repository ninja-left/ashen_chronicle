# Persistence System

The persistence system stores the world and character state while keeping inherited worlds compatible across character deaths and later versions.

## Save format

Saves use the existing JSON payload format compressed with gzip. Character-specific filenames follow the form `ashen_chronicle_save_<character>.json.gz` after filename sanitization.

Legacy uncompressed `ashen_chronicle_save.json` saves remain loadable. Save migration uses defaulted fields and explicit migration handling when new progression or time data is introduced.

## World and character boundaries

Character-specific state includes progression, conditions, and the personal quest log. World-persistent state includes faction memories, completed quest deeds, corpses, history, event cooldowns, and world time.

When a character dies, the next character can inherit the world without inheriting the dead character's personal quest log or faction reputation. Corpse contents and other persistent traces remain available to later lives.

## Saving and loading

Saving is tied to safe meditation rather than quitting. The start and load flows explicitly select compatible saves instead of silently loading a save. Character-specific save discovery preserves the selected save path.

Loading validates runtime references and reports broken or inconsistent save data clearly.

## Compatibility

Persistence changes should preserve older saves where practical. New fields should have safe defaults or explicit migration paths, and malformed compressed data should produce a stable invalid-data error rather than crashing through an unrelated failure.

## Design direction

Keep the save payload focused on persistent game state. Runtime-only campaign content should be rehydrated from the current content definitions instead of duplicated inside save files.
