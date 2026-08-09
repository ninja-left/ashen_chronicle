use crate::model::{create_inherited_state, create_new_state, EntityId, GameState, Item, WorldMode};
use crate::persistence::{load_game, save_game};
use crate::ui::{choose_from_list, pause, prompt};
use std::path::PathBuf;

#[derive(Clone, Copy)]
enum GameAction {
    Travel,
    ConfrontThreat,
    Meditate,
    Inventory,
    Journal,
    TestDeath,
    Quit,
}

#[derive(Clone, Copy)]
enum CombatAction {
    Attack,
    Guard,
    Flee,
}

struct MenuEntry {
    label: String,
    action: GameAction,
}

struct CombatEncounter {
    enemy_name: String,
    enemy_hp: i32,
    enemy_power: i32,
    enemy_id: EntityId,
}

pub fn run() -> std::io::Result<()> {
    let save_path = PathBuf::from("ashen_chronicle_save.json");
    let mut state = start_or_load(&save_path)?;
    main_loop(&mut state, &save_path)
}

fn start_or_load(save_path: &PathBuf) -> std::io::Result<GameState> {
    println!("The Ashen Chronicle v0.4.0");
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
        let menu = build_main_menu(state);
        let labels: Vec<String> = menu.iter().map(|entry| entry.label.clone()).collect();
        if let Some(choice) = choose_from_list("Choose an action", &labels, None)? {
            match menu[choice].action {
                GameAction::Travel => travel(state)?,
                GameAction::ConfrontThreat => confront_threat(state)?,
                GameAction::Meditate => meditate_and_save(state, save_path)?,
                GameAction::Inventory => show_inventory(state),
                GameAction::Journal => write_note(state)?,
                GameAction::TestDeath => force_death(state),
                GameAction::Quit => {
                    let answer = prompt("Quit without saving? [y/N] ")?;
                    if answer.eq_ignore_ascii_case("y") {
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}

fn build_main_menu(state: &GameState) -> Vec<MenuEntry> {
    let mut menu = vec![
        MenuEntry { label: "Travel".to_string(), action: GameAction::Travel },
        MenuEntry { label: "Meditate".to_string(), action: GameAction::Meditate },
        MenuEntry { label: "View inventory".to_string(), action: GameAction::Inventory },
        MenuEntry { label: "Write journal note".to_string(), action: GameAction::Journal },
        MenuEntry { label: "Test the death flow".to_string(), action: GameAction::TestDeath },
        MenuEntry { label: "Quit".to_string(), action: GameAction::Quit },
    ];

    if state.threat.active {
        menu.insert(1, MenuEntry { label: "Face threat".to_string(), action: GameAction::ConfrontThreat });
    }

    menu
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
        if location.dangerous {
            println!("Danger: this place is unsafe.");
        }
        let exits: Vec<String> = location.exits.iter().filter_map(|id| world.location_by_id(*id).map(|loc| loc.name.clone())).collect();
        println!("Exits: {}", exits.join(", "));
    }
    if state.threat.active {
        println!("Threat: {}", state.threat.label);
        println!("{}", state.threat.description);
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

    if let Some(choice) = choose_from_list("Travel where?", &options, Some("Back"))? {
        if let Some(target_id) = current_location.exits.get(choice).copied() {
            state.character.turn += 1;
            state.character.location_id = target_id;
            state.threat.clear();
            let location = state.world.location_by_id(target_id).cloned();
            let location_name = location.as_ref().map(|loc| loc.name.clone()).unwrap_or_else(|| "Unknown".to_string());
            let character_name = state.character.display_name();
            state.world.record_history(state.character.turn, format!("{} traveled to {}.", character_name, location_name));
            println!("You travel to {}.", location_name);

            if let Some(location) = location {
                if location.dangerous {
                    state.threat.activate(
                        location.id,
                        format!("{} stirs", location.name),
                        "The air is tense. Something here is still awake.".to_string(),
                    );
                    println!("This place is dangerous.");
                }
            }
        }
    }
    Ok(())
}

fn meditate_and_save(state: &mut GameState, save_path: &PathBuf) -> std::io::Result<()> {
    let location_is_dangerous = state.world.location_is_dangerous(state.character.location_id);
    if state.threat.active || location_is_dangerous {
        println!("Not safe enough to meditate here.");
        pause();
        return Ok(());
    }

    state.character.turn += 1;
    state.character.heal(3);
    let character_name = state.character.display_name();
    state.world.record_history(state.character.turn, format!("{} meditated and recovered.", character_name));
    save_game(save_path, state)?;
    println!("You settle your breathing, recover to {}/{}, and save the game.", state.character.hp, state.character.max_hp);
    Ok(())
}

fn confront_threat(state: &mut GameState) -> std::io::Result<()> {
    if !state.threat.active {
        println!("There is no active threat to face.");
        pause();
        return Ok(());
    }

    let location = match state.world.location_by_id(state.character.location_id) {
        Some(location) => location.clone(),
        None => {
            println!("The threat cannot be reached here.");
            pause();
            return Ok(());
        }
    };

    let mut encounter = CombatEncounter {
        enemy_name: format!("{} wretch", location.name),
        enemy_hp: 6,
        enemy_power: 2,
        enemy_id: state.world.allocate_id(),
    };

    println!("\nYou step into the threat.");
    println!("Enemy: {}", encounter.enemy_name);

    loop {
        if !state.character.alive {
            break;
        }
        if encounter.enemy_hp <= 0 {
            let location_id = location.id;
            let character_name = state.character.display_name();
            let enemy_name = encounter.enemy_name.clone();
            state.threat.clear();
            if let Some(loc) = state.world.location_by_id_mut(location_id) {
                loc.dangerous = false;
            }
            state.character.turn += 1;
            state.world.record_history(state.character.turn, format!("{} defeated {} at {}.", character_name, enemy_name, location.name));
            state.character.inventory.push(Item {
                id: encounter.enemy_id,
                name: format!("Trophy from {}", location.name),
                description: "A proof that the danger here was confronted and survived.".to_string(),
            });
            println!("The threat is broken. The place is quieter now.");
            break;
        }

        println!("\n{} HP: {}", encounter.enemy_name, encounter.enemy_hp);
        println!("Your HP: {}/{}", state.character.hp, state.character.max_hp);
        let choices = vec!["Attack".to_string(), "Guard".to_string(), "Flee".to_string()];
        match choose_from_list("Combat action", &choices, None)? {
            Some(0) => {
                state.character.turn += 1;
                let damage = 3;
                encounter.enemy_hp -= damage;
                let character_name = state.character.display_name();
                state.world.record_history(
                    state.character.turn,
                    format!("{} struck {} for {} damage.", character_name, encounter.enemy_name, damage),
                );
                if encounter.enemy_hp > 0 {
                    let retaliation = encounter.enemy_power;
                    take_combat_damage(state, retaliation, &encounter.enemy_name, &location.name);
                }
            }
            Some(1) => {
                state.character.turn += 1;
                let retaliation = (encounter.enemy_power - 1).max(0);
                let character_name = state.character.display_name();
                state.world.record_history(
                    state.character.turn,
                    format!("{} guarded against {}.", character_name, encounter.enemy_name),
                );
                if retaliation > 0 {
                    take_combat_damage(state, retaliation, &encounter.enemy_name, &location.name);
                } else {
                    println!("You brace yourself and hold the line.");
                }
            }
            Some(2) => {
                state.character.turn += 1;
                let character_name = state.character.display_name();
                state.world.record_history(
                    state.character.turn,
                    format!("{} fled from {} at {}.", character_name, encounter.enemy_name, location.name),
                );
                println!("You back away and the threat remains.");
                break;
            }
            _ => {}
        }

        if state.character.hp <= 0 {
            state.character.alive = false;
            let character_name = state.character.display_name();
            state.world.record_history(
                state.character.turn,
                format!("{} fell to {} at {}.", character_name, encounter.enemy_name, location.name),
            );
            println!("You were overwhelmed.");
            break;
        }
    }

    Ok(())
}

fn take_combat_damage(state: &mut GameState, damage: i32, enemy_name: &str, location_name: &str) {
    if damage <= 0 {
        println!("The blow glances off harmlessly.");
        return;
    }

    state.character.hp -= damage;
    let character_name = state.character.display_name();
    state.world.record_history(
        state.character.turn,
        format!("{} took {} damage from {} at {}.", character_name, damage, enemy_name, location_name),
    );
    println!("You take {} damage.", damage);
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
        let character_name = state.character.display_name();
        state.world.record_history(state.character.turn, format!("{} noted: {}", character_name, note));
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
    state.world.record_history(
        state.character.turn,
        format!("{} died at {}.", state.character.display_name(), location_name),
    );
    println!("The character falls.");
}

fn death_screen(state: &mut GameState, save_path: &PathBuf) -> std::io::Result<bool> {
    println!("\n{} has died.", state.character.display_name());
    let options = vec![
        "Create a new world".to_string(),
        "Inherit this world with a new character".to_string(),
        "Save and quit".to_string(),
    ];
    match choose_from_list("Death screen", &options, None)? {
        Some(0) => {
            *state = create_from_prompts(WorldMode::New)?;
            Ok(true)
        }
        Some(1) => {
            *state = create_inherited_from_world(state)?;
            Ok(true)
        }
        Some(2) => {
            save_game(save_path, state)?;
            println!("Saved to {}", save_path.display());
            Ok(false)
        }
        _ => Ok(false),
    }
}
