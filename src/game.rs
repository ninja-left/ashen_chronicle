use crate::model::{create_inherited_state, create_new_state, GameState, WorldMode};
use crate::persistence::{load_game, save_game};
use crate::ui::{choose_from_list, pause, prompt};
use std::path::PathBuf;

pub fn run() -> std::io::Result<()> {
    let save_path = PathBuf::from("ashen_chronicle_save.json");
    let mut state = start_or_load(&save_path)?;
    main_loop(&mut state, &save_path)
}

fn start_or_load(save_path: &PathBuf) -> std::io::Result<GameState> {
    println!("The Ashen Chronicle v0.2.0");
    println!("--------------------------------");
    if save_path.exists() {
        let choice = prompt("Load existing save? [y/N] ")?;
        if choice.eq_ignore_ascii_case("y") {
            match load_game(save_path) {
                Ok(state) => return Ok(state),
                Err(err) => {
                    println!("Could not load save: {err}");
                    println!("Starting a new world instead.");
                }
            }
        }
    }
    create_from_prompts(WorldMode::New)
}

fn create_from_prompts(mode: WorldMode) -> std::io::Result<GameState> {
    let world_name = prompt("World name [The Ashen Crown]: ")?;
    let world_name = if world_name.is_empty() { "The Ashen Crown".to_string() } else { world_name };
    let character_name = prompt("Character name [Warden]: ")?;
    let character_name = if character_name.is_empty() { "Warden".to_string() } else { character_name };
    let title = prompt("Character title [Ashborn]: ")?;
    let title = if title.is_empty() { "Ashborn".to_string() } else { title };
    Ok(create_new_state(&world_name, mode, character_name, title))
}

fn create_inherited_from_world(state: &GameState) -> std::io::Result<GameState> {
    let character_name = prompt("New character name [Warden]: ")?;
    let character_name = if character_name.is_empty() { "Warden".to_string() } else { character_name };
    let title = prompt("New character title [Ashborn]: ")?;
    let title = if title.is_empty() { "Ashborn".to_string() } else { title };
    Ok(create_inherited_state(state.world.clone(), character_name, title))
}

fn main_loop(state: &mut GameState, save_path: &PathBuf) -> std::io::Result<()> {
    loop {
        if !state.character.alive {
            if !death_screen(state, save_path)? {
                break;
            }
            continue;
        }

        render_state(state);
        let options = vec![
            "Travel".to_string(),
            "Rest".to_string(),
            "View inventory".to_string(),
            "Write journal note".to_string(),
            "Test the death flow".to_string(),
            "Save game".to_string(),
            "Quit".to_string(),
        ];
        match choose_from_list("Choose an action", &options)? {
            Some(0) => travel(state)?,
            Some(1) => rest(state),
            Some(2) => show_inventory(state),
            Some(3) => write_note(state)?,
            Some(4) => force_death(state),
            Some(5) => save_now(state, save_path)?,
            Some(6) | None => {
                let answer = prompt("Quit without saving? [y/N] ")?;
                if answer.eq_ignore_ascii_case("y") {
                    break;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn render_state(state: &GameState) {
    let world = &state.world;
    let character = &state.character;
    let location = world.location_by_id(character.location_id);
    println!("\n=== {} ===", world.name);
    println!("World mode: {:?}", world.mode);
    println!("Character: {}", character.display_name());
    println!("HP: {}/{}", character.hp, character.max_hp);
    if let Some(location) = location {
        let region_name = world.region_by_id(location.region_id).map(|region| region.name.as_str()).unwrap_or("Unknown region");
        println!("Location: {} ({})", location.name, region_name);
        println!("{}", location.description);
        let exits: Vec<String> = location
            .exits
            .iter()
            .filter_map(|id| world.location_by_id(*id).map(|loc| loc.name.clone()))
            .collect();
        println!("Exits: {}", exits.join(", "));
    }
    println!("History entries: {}", world.history.len());
}

fn travel(state: &mut GameState) -> std::io::Result<()> {
    let current_location = match state.world.location_by_id(state.character.location_id) {
        Some(location) => location.clone(),
        None => {
            println!("You are lost in a location that no longer exists.");
            pause();
            return Ok(());
        }
    };

    let options: Vec<String> = current_location
        .exits
        .iter()
        .filter_map(|id| state.world.location_by_id(*id).map(|loc| loc.name.clone()))
        .collect();
    if options.is_empty() {
        println!("There is nowhere to travel.");
        pause();
        return Ok(());
    }

    if let Some(choice) = choose_from_list("Travel where?", &options)? {
        if let Some(target_id) = current_location.exits.get(choice).copied() {
            state.character.location_id = target_id;
            state.character.turn += 1;
            let location_name = state
                .world
                .location_by_id(target_id)
                .map(|loc| loc.name.clone())
                .unwrap_or_else(|| "Unknown".to_string());
            state.world.record_history(state.character.turn, format!("{} traveled to {}.", state.character.display_name(), location_name));
            println!("You travel to {}.", location_name);
        }
    }
    Ok(())
}

fn rest(state: &mut GameState) {
    state.character.turn += 1;
    state.character.hp = (state.character.hp + 3).min(state.character.max_hp);
    state.world.record_history(state.character.turn, format!("{} rested and recovered.", state.character.display_name()));
    println!("You rest. HP is now {}/{}.", state.character.hp, state.character.max_hp);
}

fn show_inventory(state: &GameState) {
    println!("\nInventory for {}", state.character.display_name());
    if state.character.inventory.is_empty() {
        println!("  Nothing.");
    } else {
        for item in &state.character.inventory {
            println!("  - {}: {}", item.name, item.description);
        }
    }
    pause();
}

fn write_note(state: &mut GameState) -> std::io::Result<()> {
    let note = prompt("Write a journal note: ")?;
    if !note.is_empty() {
        state.character.notes.push(note.clone());
        state.character.turn += 1;
        state.world.record_history(state.character.turn, format!("{} noted: {}", state.character.display_name(), note));
    }
    Ok(())
}

fn force_death(state: &mut GameState) {
    state.character.hp = 0;
    state.character.alive = false;
    state.character.turn += 1;
    let location_name = state
        .world
        .location_by_id(state.character.location_id)
        .map(|location| location.name.clone())
        .unwrap_or_else(|| "an unknown place".to_string());
    state.world.record_history(state.character.turn, format!("{} died at {}.", state.character.display_name(), location_name));
    println!("The character falls.");
}

fn save_now(state: &GameState, save_path: &PathBuf) -> std::io::Result<()> {
    save_game(save_path, state)?;
    println!("Saved to {}", save_path.display());
    Ok(())
}

fn death_screen(state: &mut GameState, save_path: &PathBuf) -> std::io::Result<bool> {
    println!("\n{} has died.", state.character.display_name());
    let options = vec![
        "Create a new world".to_string(),
        "Inherit this world with a new character".to_string(),
        "Save and quit".to_string(),
    ];
    match choose_from_list("Death screen", &options)? {
        Some(0) => {
            *state = create_from_prompts(WorldMode::New)?;
            Ok(true)
        }
        Some(1) => {
            *state = create_inherited_from_world(state)?;
            Ok(true)
        }
        Some(2) | None => {
            save_now(state, save_path)?;
            Ok(false)
        }
        _ => Ok(false),
    }
}
