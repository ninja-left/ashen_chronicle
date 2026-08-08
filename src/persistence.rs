use crate::model::GameState;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

const SAVE_FILE_VERSION: u32 = 1;

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
    if parsed.save_file_version != SAVE_FILE_VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Save file version {} is not supported by this build",
                parsed.save_file_version
            ),
        ));
    }
    Ok(parsed.game)
}
