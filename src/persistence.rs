use crate::model::GameState;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

const SAVE_FILE_VERSION: u32 = 2;

#[derive(Debug, Serialize, Deserialize)]
struct SaveFile {
    save_file_version: u32,
    game: GameState,
}

pub fn save_game(path: &Path, state: &GameState) -> io::Result<()> {
    let payload = SaveFile {
        save_file_version: SAVE_FILE_VERSION,
        game: state.clone(),
    };
    let json = serde_json::to_string_pretty(&payload)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;
    fs::write(path, json)
}

pub fn load_game(path: &Path) -> io::Result<GameState> {
    let data = fs::read_to_string(path)?;
    let parsed: SaveFile = serde_json::from_str(&data)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?;
    if parsed.save_file_version > SAVE_FILE_VERSION || parsed.save_file_version == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Save file version {} is not supported by this build",
                parsed.save_file_version
            ),
        ));
    }
    let mut game = parsed.game;
    if parsed.save_file_version < 2 {
        // v1 did not store world time, progression, or conditions. Keep the
        // old character/world intact and start the new systems at sensible defaults.
        game.world.time_points = 3;
        game.world.day = 1;
        if game.character.level == 0 { game.character.level = 1; }
        if game.character.attributes.might == 0 && game.character.attributes.insight == 0 && game.character.attributes.endurance == 0 {
            game.character.attributes.might = 1;
            game.character.attributes.insight = 1;
            game.character.attributes.endurance = 1;
        }
    }
    Ok(game)
}
