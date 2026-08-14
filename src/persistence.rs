use crate::model::GameState;
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

const SAVE_FILE_VERSION: u32 = 2;
pub const SAVE_BASE_NAME: &str = "ashen_chronicle_save";
const SAVE_EXTENSION: &str = "json.gz";
const LEGACY_SAVE_FILE_NAME: &str = "ashen_chronicle_save.json";

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
    let json = serde_json::to_vec_pretty(&payload)
        .map_err(|err| io::Error::other(err.to_string()))?;

    let file = File::create(path)?;
    let mut encoder = GzEncoder::new(file, Compression::default());
    encoder.write_all(&json)?;
    encoder.finish()?;
    Ok(())
}

pub fn load_game(path: &Path) -> io::Result<GameState> {
    let mut data = Vec::new();
    File::open(path)?.read_to_end(&mut data)?;
    let json = if is_gzip(&data) {
        let mut decoder = GzDecoder::new(data.as_slice());
        let mut decoded = Vec::new();
        decoder
            .read_to_end(&mut decoded)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?;
        decoded
    } else {
        // Backward compatibility for the pre-compression JSON save format.
        data
    };

    let parsed: SaveFile = serde_json::from_slice(&json)
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
        game.world.time_points = 3;
        game.world.day = 1;
        if game.character.level == 0 {
            game.character.level = 1;
        }
        if game.character.attributes.might == 0
            && game.character.attributes.insight == 0
            && game.character.attributes.endurance == 0
        {
            game.character.attributes.might = 1;
            game.character.attributes.insight = 1;
            game.character.attributes.endurance = 1;
        }
    }
    game.campaign_content = Some(crate::content::load_campaign_content());
    Ok(game)
}

pub fn character_save_path(directory: &Path, character_name: &str) -> PathBuf {
    directory.join(format!(
        "{}_{}.{}",
        SAVE_BASE_NAME,
        sanitize_filename_component(character_name),
        SAVE_EXTENSION
    ))
}

pub fn find_save_files(directory: &Path) -> io::Result<Vec<PathBuf>> {
    let mut saves = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if file_name.starts_with(&format!("{}_", SAVE_BASE_NAME))
            && file_name.ends_with(&format!(".{}", SAVE_EXTENSION))
        {
            saves.push(path);
        }
    }
    saves.sort();
    Ok(saves)
}

pub fn legacy_save_path(directory: &Path) -> PathBuf {
    directory.join(LEGACY_SAVE_FILE_NAME)
}

pub fn sanitize_filename_component(name: &str) -> String {
    let mut sanitized = String::new();
    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ' ') {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
        }
    }
    let sanitized = sanitized.trim().trim_matches('.').to_string();
    let has_safe_name_char = sanitized.chars().any(|ch| ch.is_ascii_alphanumeric());
    if sanitized.is_empty() || !has_safe_name_char {
        "unnamed".to_string()
    } else {
        sanitized
    }
}

fn is_gzip(data: &[u8]) -> bool {
    data.len() >= 2 && data[0] == 0x1f && data[1] == 0x8b
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{create_new_state, EventCooldown, WorldMode};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir() -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("ashen_chronicle_save_tests_{}", stamp));
        fs::create_dir_all(&path).expect("temporary directory should be created");
        path
    }

    #[test]
    fn compressed_save_round_trips() {
        let dir = temp_dir();
        let path = character_save_path(&dir, "Ash Walker");
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
        let bytes = fs::read(&path).expect("save should exist");
        assert!(is_gzip(&bytes));

        let loaded = load_game(&path).expect("load should succeed");
        assert_eq!(loaded.character.turn, 7);
        assert_eq!(loaded.character.name, "Tester");
        assert_eq!(loaded.world.event_cooldowns, state.world.event_cooldowns);
        fs::remove_dir_all(&dir).expect("temporary directory should be removed");
    }

    #[test]
    fn legacy_json_save_still_loads() {
        let dir = temp_dir();
        let path = legacy_save_path(&dir);
        let state = create_new_state(
            "Test World",
            WorldMode::New,
            "Tester".to_string(),
            "Ash Walker".to_string(),
        );
        let payload = SaveFile {
            save_file_version: SAVE_FILE_VERSION,
            game: state.clone(),
        };
        fs::write(
            &path,
            serde_json::to_vec_pretty(&payload).expect("json should serialize"),
        )
        .expect("legacy save should be written");

        let loaded = load_game(&path).expect("legacy save should load");
        assert_eq!(loaded.character.name, state.character.name);
        fs::remove_dir_all(&dir).expect("temporary directory should be removed");
    }

    #[test]
    fn sanitize_filename_component_is_filesystem_safe() {
        assert_eq!(
            sanitize_filename_component("Ash/Walker:Night"),
            "Ash_Walker_Night"
        );
        assert_eq!(sanitize_filename_component("..."), "unnamed");
        assert_eq!(sanitize_filename_component("  Geralt  "), "Geralt");
    }

    #[test]
    fn character_save_path_appends_character_name() {
        let path = character_save_path(Path::new("."), "Ash Walker");
        assert_eq!(
            path,
            PathBuf::from("./ashen_chronicle_save_Ash Walker.json.gz")
        );
    }

    #[test]
    fn invalid_gzip_is_reported_as_invalid_data() {
        let dir = temp_dir();
        let path = dir.join("broken.json.gz");
        fs::write(&path, [0x1f, 0x8b, 0x00, 0x01]).expect("broken gzip should be written");

        let err = load_game(&path).expect_err("invalid gzip should fail");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        fs::remove_dir_all(&dir).expect("temporary directory should be removed");
    }
}
