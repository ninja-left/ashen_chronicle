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


#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{create_new_state, EventCooldown, WorldMode};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_save_path() -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be valid")
            .as_nanos();
        std::env::temp_dir().join(format!("ashen_chronicle_save_test_{}_{}.json", std::process::id(), stamp))
    }

    #[test]
    fn event_cooldowns_survive_save_and_load() {
        let path = temp_save_path();
        let mut state = create_new_state(
            "Test World",
            WorldMode::New,
            "Tester".to_string(),
            "Ash Walker".to_string(),
        );
        state.character.turn = 7;
        state.world.event_cooldowns.push(EventCooldown {
            event_id: "travel.ruined-road".to_string(),
            ready_at_turn: 11,
        });

        save_game(&path, &state).expect("save should succeed");
        let loaded = load_game(&path).expect("load should succeed");
        let _ = std::fs::remove_file(&path);

        assert_eq!(loaded.character.turn, 7);
        assert_eq!(
            loaded.world.event_cooldowns,
            vec![EventCooldown {
                event_id: "travel.ruined-road".to_string(),
                ready_at_turn: 11,
            }]
        );
    }
}
