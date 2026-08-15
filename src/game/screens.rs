use crate::model::{create_inherited_state, create_new_state, GameState, WorldMode};
use crate::persistence::{character_save_path, find_save_files, legacy_save_path, load_game};
use crate::ui::{choose_from_list, narrate, pause, prompt, set_menu_screen};
use crate::game::validate_loaded_state;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

macro_rules! println {
    () => {
        crate::ui::line("");
    };
    ($($arg:tt)*) => {
        crate::ui::line(&format!($($arg)*))
    };
}

pub(crate) fn start_screen() -> std::io::Result<Option<(GameState, PathBuf)>> {
    const VARIANTS: [(&str, &str); 4] = [
        (
            "The road is quiet. Something is listening.",
            r#"             .-.
            /   \\
           /     \\
      _____/       \\
         \\   /\\   /
          \\ /  \\ /
           Y    Y
          /      \\
         /        \\
        /          \\
       /            \\
      /              \\
     /                \\
"#,
        ),
        (
            "The old gods are silent. The stones remember.",
            r#"             /\\
            /  \\
           /____\\
          |      |
      _____|      |_____
          /        \\
         /          \\
        /            \\
       /              \\
      /________________\\
            ||  ||
            ||  ||
            ||  ||
        ____||__||____
"#,
        ),
        (
            "Only ash remains where the fire once lived.",
            r#"              .-.
             (   )
              `-'
             /   \\
            /_____\\

              ||
             /  \\
            /____\\
           |      |
           |      |
           |______|
"#,
        ),
        (
            "You are not the first to walk this road.",
            r#"                 .-.
              .-'   '-.
            .'         '.
           /    .---.    \\
          |    /     \\    |
          |   |  o o  |   |
          |   |   ^   |   |
           \\   \\ '-' /   /
            '.  '---'  .'
              '-.____.-'
                  ||
                  ||
             _____||_____
"#,
        ),
    ];

    loop {
        let current_dir = PathBuf::from(".");
        let save_files = find_save_files(&current_dir)?;
        let legacy_path = legacy_save_path(&current_dir);
        let has_saves = !save_files.is_empty() || legacy_path.exists();
        let tick = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.subsec_nanos() as usize)
            .unwrap_or(0);
        let (sentence, art) = VARIANTS[tick % VARIANTS.len()];

        set_menu_screen(
            "THE ASHEN CHRONICLE",
            Some(sentence.to_string()),
            Some(art.to_string()),
        );

        let mut options = vec!["New Game".to_string()];
        if has_saves {
            options.push("Load Game".to_string());
        }
        options.push("Quit".to_string());

        let Some(choice) = choose_from_list("Begin", &options, None)? else {
            continue;
        };

        match options[choice].as_str() {
            "New Game" => {
                set_menu_screen(
                    "NEW GAME",
                    Some("Begin a new life in a world that has yet to remember you.".to_string()),
                    None,
                );
                let state = create_from_prompts(WorldMode::New)?;
                let save_path = character_save_path(&current_dir, &state.character.name);
                return Ok(Some((state, save_path)));
            }
            "Load Game" => {
                if let Some(result) = load_screen(&current_dir, save_files, legacy_path)? {
                    return Ok(Some(result));
                }
            }
            "Quit" => return Ok(None),
            _ => {}
        }
    }
}

fn load_screen(
    current_dir: &Path,
    mut save_files: Vec<PathBuf>,
    legacy_path: PathBuf,
) -> std::io::Result<Option<(GameState, PathBuf)>> {
    if legacy_path.exists() && !save_files.iter().any(|path| path == &legacy_path) {
        save_files.push(legacy_path);
    }

    if save_files.is_empty() {
        return Ok(None);
    }

    let options: Vec<String> = save_files
        .iter()
        .map(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Unknown save")
                .to_string()
        })
        .collect();

    set_menu_screen(
        "LOAD GAME",
        Some("Choose a life to continue.".to_string()),
        None,
    );

    let Some(choice) = choose_from_list("Saved lives", &options, Some("Back"))? else {
        return Ok(None);
    };

    let path = &save_files[choice];
    match load_game(path) {
        Ok(state) => {
            let warnings = validate_loaded_state(&state);
            let save_path = character_save_path(current_dir, &state.character.name);
            if warnings.is_empty() {
                return Ok(Some((state, save_path)));
            }
            let warning_text = format!(
                "Save loaded with {} warning(s). The game will continue, but the save should be reviewed.",
                warnings.len()
            );
            set_menu_screen("LOAD GAME", Some(warning_text), None);
            pause();
            Ok(Some((state, save_path)))
        }
        Err(err) => {
            set_menu_screen(
                "LOAD GAME",
                Some(format!("Could not load {}: {}", path.display(), err)),
                None,
            );
            pause();
            Ok(None)
        }
    }
}

fn create_from_prompts(mode: WorldMode) -> std::io::Result<GameState> {
    let world_name = if matches!(&mode, WorldMode::New) {
        let input = prompt("Name the world [The Ashen Crown]: ")?;
        if input.is_empty() {
            "The Ashen Crown".to_string()
        } else {
            input
        }
    } else {
        "The Ashen Crown".to_string()
    };
    let character_name = prompt("Character name: ")?;
    let title = prompt("Character title [Ash Walker]: ")?;
    let character_name = if character_name.is_empty() {
        "Wanderer".to_string()
    } else {
        character_name
    };
    let title = if title.is_empty() {
        "Ash Walker".to_string()
    } else {
        title
    };
    Ok(create_new_state(&world_name, mode, character_name, title))
}

fn create_inherited_from_world(state: &GameState) -> std::io::Result<GameState> {
    let character_name = prompt("New character name: ")?;
    let title = prompt("New character title [Ash Walker]: ")?;
    let character_name = if character_name.is_empty() {
        "Heir".to_string()
    } else {
        character_name
    };
    let title = if title.is_empty() {
        "Ash Walker".to_string()
    } else {
        title
    };
    Ok(create_inherited_state(state, character_name, title))
}

pub(crate) fn quit_screen() -> std::io::Result<bool> {
    const VARIANTS: [(&str, &str, &str, &str); 9] = [
        (
            "The road ends here.\nFor tonight, anyway.",
            "Let the ashes take it.",
            "Not yet. The night has more to say.",
            r#"        .-''''-.
       /  .--.  \\
      /  /    \\  \\
      | |      | |
      | |      | |
      |  \\____/  |
       \\        /
        '------'
"#,
        ),
        (
            "The fire is dying.\nYour story does not have to.",
            "Close the book.",
            "Turn the page.",
            r#"          /\\
         /  \\
        / /\\ \\
       / /  \\ \\
      /_/____\\_\\
        ||  ||
        ||  ||
        ||  ||
       _||__||_
"#,
        ),
        (
            "Night has swallowed the road.\nOnly your footprints remain.",
            "Leave them to the dark.",
            "Keep walking.",
            r#"       _..._       _..._
     .-'     '-. .-'     '-'.
    /           V           \\
   /      _           _      \\
   |     (_)         (_)     |
   |          .---.          |
    \\        /     \\        /
     '-._____'-----'_____.-'
"#,
        ),
        (
            "The last ember has gone black.\nThe silence is waiting.",
            "Let it be silent.",
            "Break the silence.",
            r#"            .
           / \\
          /   \\
         /_____\\
         |     |
         | RIP |
         |     |
         |_____|
"#,
        ),
        (
            "The gate closes behind you. The road will remain.",
            "Close the gate.",
            "Leave it open.",
            r#"        ______________________
       /|                    |\\
      / |                    | \\
     /  |                    |  \\
    /   |                    |   \\
   /    |                    |    \\
  /_____|____________________|_____\\
        |                    |
        |        ____        |
        |       |    |       |
        |       |    |       |
        |_______|____|_______|
"#,
        ),
        (
            "The flame is gone. The silence remains.",
            "Let the silence remain.",
            "Feed the flame again.",
            r#"             /\\
            /  \\
           /____\\
          |      |
          |  __  |
          | |  | |
          | |__| |
          |______|
             ||
          ___||___
         |        |
         |  .  .  |
         |________|
"#,
        ),
        (
            "The road continues without you.",
            "Leave the road behind.",
            "Keep walking.",
            r#"             /\\                 /\\
            /  \\               /  \\
           /    \\             /    \\
          /      \\___________/      \\
         /                         \\
        /                           \\
       /_____________________________\\
                    ||
                    ||
                    ||
                    ||
"#,
        ),
        (
            "For now, the dead can wait.",
            "Let the dead wait.",
            "Not tonight.",
            r#"       _        _        _
      | |      | |      | |
     _| |__   _| |__   _| |__
    /     \\  /     \\  /     \\
   /       \\/       \\/       \\
        |     |     |
        |     |     |
   _____|_____|_____|_____
"#,
        ),
        (
            "One last look. Then darkness.",
            "One last look.",
            "Stay a little longer.",
            r#"             .       *
        *          .
                  .       *
           _____________
          /             \\
         /               \\
        /                 \\
       /                   \\
      /                     \\
     /_______________________\\
             ||   ||
             ||   ||
             ||   ||
"#,
        ),
    ];

    let tick = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.subsec_nanos() as usize)
        .unwrap_or(0);
    let index = tick % VARIANTS.len();
    let (line, leave_choice, stay_choice, art) = VARIANTS[index];
    let options = vec![leave_choice.to_string(), stay_choice.to_string()];

    set_menu_screen("LEAVE?", Some(line.to_string()), Some(art.to_string()));
    match choose_from_list("", &options, None)? {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Ok(false),
    }
}

pub(crate) fn death_screen(state: &mut GameState) -> std::io::Result<bool> {
    death_legacy_screen(state);
    let options = vec![
        "Create a new world".to_string(),
        "Inherit this world with a new character".to_string(),
        "Quit".to_string(),
    ];
    match choose_from_list("What remains?", &options, None)? {
        Some(0) => {
            set_menu_screen(
                "NEW GAME",
                Some("A new life begins, but this world remembers what happened here.".to_string()),
                None,
            );
            *state = create_from_prompts(WorldMode::New)?;
            Ok(true)
        }
        Some(1) => {
            set_menu_screen(
                "INHERIT THIS WORLD",
                Some("The next life will inherit the world, not the memories.".to_string()),
                None,
            );
            *state = create_inherited_from_world(state)?;
            Ok(true)
        }
        Some(2) => quit_screen(),
        _ => Ok(false),
    }
}

fn death_legacy_screen(state: &GameState) {
    const VARIANTS: [(&str, &str); 8] = [
        (
            "The armor remains. The one inside does not.",
            r#"           _________
          /         \\
         /  _     _  \\
        |  / \\   / \\  |
        |  \\_/   \\_/  |
         \\     ^     /
          \\_________/
             /|\\
            / | \\
           /  |  \\
          /   |   \\
         /    |    \\
              |
             / \\
            /   \\
 "#,
        ),
        (
            "No crown survives the grave.",
            r#"          /\\        /\\
         /  \\  /\\  /  \\
        /____\\/  \\/____\\
             \\  /
              \\\/
              ||
              ||
          ____||____
         /          \\
        /____________\\
"#,
        ),
        (
            "The earth remembers what you leave behind.",
            r#"              ______
             /      \\
            /        \\
           /          \\
          /            \\
         /______________\\
              ||||
              ||||
              ||||
        ______||||______
       /                \\
      /__________________\\
"#,
        ),
        (
            "The body falls. The world does not.",
            r#"             /\\
            /  \\
           /    \\
          /______\\
             ||
        _____||_____
       /     ||     \\
      /      ||      \\
     /_______||_______\\
             ||
            /  \\
           /    \\
          /______\\
 "#,
        ),
        (
            "The flame gutters. The ash remains.",
            r#"              |
             / \\
            /   \\
           |     |
           |     |
           |_____|
             |||
             |||
          ___|||___
         /         \\
        /___________\\

             . . .
 "#,
        ),
        (
            "The body is still. The world is not.",
            r#"        .-''''-.
      .'  .--.  '.
     /   /    \\   \\
    |   |      |   |
    |   |      |   |
     \\   \\____/   /
      '.        .'
        '-.__.-'
"#,
        ),
        (
            "Your footprints end here. What you changed does not.",
            r#"          .-.
         /   \\
        | RIP |
        |     |
        |_____|
          ||
       ___||___
"#,
        ),
        (
            "One life has gone into the ash. The road remembers.",
            r#"          _  _
        _| || |_
       |_  __  _|
         | || |
         | || |
        _|_||_|_
"#,
        ),
    ];
    let tick = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.subsec_nanos() as usize)
        .unwrap_or(0);
    let (line, art) = VARIANTS[tick % VARIANTS.len()];
    let character_name = state.character.display_name();
    let location_name = state
        .world
        .location_by_id(state.character.location_id)
        .map(|location| location.name.as_str())
        .unwrap_or("an unknown place");

    println!("\n{art}");
    println!("{line}");
    println!(
        "\n{} died at {} on turn {}.",
        character_name, location_name, state.character.turn
    );

    let completed: Vec<&str> = state
        .world
        .history
        .iter()
        .filter(|entry| entry.text.contains(&character_name) && entry.text.contains("completed "))
        .map(|entry| entry.text.as_str())
        .collect();
    println!("\nDeeds remembered:");
    if completed.is_empty() {
        println!("  None recorded.");
    } else {
        for deed in completed.iter().take(5) {
            println!("  - {}", deed);
        }
    }

    println!("\nFaction standing at death:");
    if state.factions.is_empty() {
        println!("  None recorded.");
    } else {
        for faction in &state.factions {
            println!("  - {} {:+}", faction.name, faction.reputation);
        }
    }

    let dropped = state
        .corpses
        .last()
        .map(|corpse| {
            corpse
                .inventory
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    println!("\nWhat remains on the body:");
    if dropped.is_empty() {
        println!("  Nothing worth carrying.");
    } else {
        println!("  {}", dropped.join(", "));
    }
    println!("\nThe next life will know none of this as memory. It can only be discovered.");
    pause();
}
