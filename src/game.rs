use crate::model::{
    create_inherited_state, create_new_state, Corpse, EntityId, Faction, GameState, Item, Npc,
    Quest, WorldMode,
};
use crate::persistence::{load_game, save_game};
use crate::ui::{choose_from_list, narrate, pause, prompt};
use std::mem;
use std::path::PathBuf;

#[derive(Clone, Copy)]
enum GameAction {
    Travel,
    ConfrontThreat,
    SearchRemains,
    Meditate,
    QuestLog,
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
    bootstrap_campaign_content(&mut state);
    main_loop(&mut state, &save_path)
}

fn start_or_load(save_path: &PathBuf) -> std::io::Result<GameState> {
    println!("The Ashen Chronicle v0.7.0");
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
    let mut state = create_new_state(&world_name, mode, character_name, title);
    bootstrap_campaign_content(&mut state);
    Ok(state)
}

fn create_inherited_from_world(state: &GameState) -> std::io::Result<GameState> {
    let character_name = prompt("New character name [Warden]: ")?;
    let character_name = if character_name.is_empty() { "Warden".to_string() } else { character_name };
    let title = prompt("New character title [Ashborn]: ")?;
    let title = if title.is_empty() { "Ashborn".to_string() } else { title };
    let mut inherited = create_inherited_state(state, character_name, title);
    bootstrap_campaign_content(&mut inherited);
    Ok(inherited)
}

fn bootstrap_campaign_content(state: &mut GameState) {
    ensure_demo_factions(state);
    ensure_demo_npcs(state);
    ensure_demo_quests(state);
}

fn ensure_demo_factions(state: &mut GameState) {
    if faction_by_name(state, "Cinder Wardens").is_none() {
        let id = state.world.allocate_id();
        state.factions.push(Faction::new(id, "Cinder Wardens"));
    }

    if faction_by_name(state, "Hollow Market Kin").is_none() {
        let id = state.world.allocate_id();
        state.factions.push(Faction::new(id, "Hollow Market Kin"));
    }
}

fn ensure_demo_npcs(state: &mut GameState) {
    if npc_by_name(state, "Mira").is_none() {
        if let (Some(market_id), Some(faction_id)) = (
            location_id_by_name(&state.world, "Hollow Market"),
            faction_id_by_name(state, "Cinder Wardens"),
        ) {
            let id = state.world.allocate_id();
            let mut npc = Npc::new(id, "Mira", "Scout", market_id, Some(faction_id));
            npc.memory.push("Keeps watch on the shrine road.".to_string());
            state.npcs.push(npc);
        }
    }

    if npc_by_name(state, "Bram").is_none() {
        if let (Some(gate_id), Some(faction_id)) = (
            location_id_by_name(&state.world, "Ashen Gate"),
            faction_id_by_name(state, "Hollow Market Kin"),
        ) {
            let id = state.world.allocate_id();
            let mut npc = Npc::new(id, "Bram", "Gatekeeper", gate_id, Some(faction_id));
            npc.memory.push("Counts every traveler who passes the gate.".to_string());
            state.npcs.push(npc);
        }
    }
}

fn ensure_demo_quests(state: &mut GameState) {
    if state.quests.is_empty() {
        if let (Some(shrine_id), Some(faction_id)) = (
            location_id_by_name(&state.world, "Old Shrine"),
            faction_id_by_name(state, "Cinder Wardens"),
        ) {
            let id = state.world.allocate_id();
            state.quests.push(Quest::new(
                id,
                "Quiet the Old Shrine",
                "The wardens want the shrine cleared of whatever woke there.",
                shrine_id,
                faction_id,
            ));
        }
    }
}

fn faction_by_name<'a>(state: &'a GameState, name: &str) -> Option<&'a Faction> {
    state.factions.iter().find(|faction| faction.name == name)
}

fn faction_by_id<'a>(state: &'a GameState, faction_id: EntityId) -> Option<&'a Faction> {
    state.factions.iter().find(|faction| faction.id == faction_id)
}

fn faction_by_id_mut<'a>(state: &'a mut GameState, faction_id: EntityId) -> Option<&'a mut Faction> {
    state.factions.iter_mut().find(|faction| faction.id == faction_id)
}

fn faction_id_by_name(state: &GameState, name: &str) -> Option<EntityId> {
    faction_by_name(state, name).map(|faction| faction.id)
}

fn npc_by_name<'a>(state: &'a GameState, name: &str) -> Option<&'a Npc> {
    state.npcs.iter().find(|npc| npc.name == name)
}

fn npc_ids_at_location(state: &GameState, location_id: EntityId) -> Vec<EntityId> {
    state
        .npcs
        .iter()
        .filter(|npc| npc.location_id == location_id)
        .map(|npc| npc.id)
        .collect()
}

fn location_id_by_name(world: &crate::model::World, name: &str) -> Option<EntityId> {
    world.locations.iter().find(|location| location.name == name).map(|location| location.id)
}

fn npc_index_by_id(state: &GameState, npc_id: EntityId) -> Option<usize> {
    state.npcs.iter().position(|npc| npc.id == npc_id)
}

fn quest_by_target_location_mut(state: &mut GameState, location_id: EntityId) -> Option<&mut Quest> {
    state.quests.iter_mut().find(|quest| quest.target_location_id == location_id)
}

fn quest_by_title_mut<'a>(state: &'a mut GameState, title: &str) -> Option<&'a mut Quest> {
    state.quests.iter_mut().find(|quest| quest.title == title)
}

fn main_loop(state: &mut GameState, save_path: &PathBuf) -> std::io::Result<()> {
    loop {
        if !state.character.alive {
            if !death_screen(state, save_path)? {
                break;
            }
            state.last_announced_location_id = None;
            continue;
        }

        render_state(state);
        maybe_run_location_scene(state)?;
        let menu = build_main_menu(state);
        let labels: Vec<String> = menu.iter().map(|entry| entry.label.clone()).collect();
        if let Some(choice) = choose_from_list("Choose an action", &labels, None)? {
            match menu[choice].action {
                GameAction::Travel => travel(state)?,
                GameAction::ConfrontThreat => confront_threat(state)?,
                GameAction::SearchRemains => search_remains(state)?,
                GameAction::Meditate => meditate_and_save(state, save_path)?,
                GameAction::QuestLog => review_quests(state),
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
        MenuEntry { label: "Quest log".to_string(), action: GameAction::QuestLog },
        MenuEntry { label: "View inventory".to_string(), action: GameAction::Inventory },
        MenuEntry { label: "Write journal note".to_string(), action: GameAction::Journal },
        MenuEntry { label: "Test the death flow".to_string(), action: GameAction::TestDeath },
        MenuEntry { label: "Quit".to_string(), action: GameAction::Quit },
    ];

    if state.threat.active {
        menu.insert(1, MenuEntry { label: "Face threat".to_string(), action: GameAction::ConfrontThreat });
    }

    if has_unscavenged_remains_at_location(state) {
        let insert_at = if state.threat.active { 2 } else { 1 };
        menu.insert(insert_at, MenuEntry { label: "Search remains".to_string(), action: GameAction::SearchRemains });
    }

    menu
}

fn has_unscavenged_remains_at_location(state: &GameState) -> bool {
    let location_id = state.character.location_id;
    state
        .corpses
        .iter()
        .any(|corpse| corpse.location_id == location_id && !corpse.inventory.is_empty())
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
        let people_here: Vec<String> = state
            .npcs
            .iter()
            .filter(|npc| npc.location_id == location.id)
            .map(|npc| npc.display_name())
            .collect();
        if !people_here.is_empty() {
            println!("People here: {}", people_here.join(", "));
        }
        let remains = corpses_at_location(state, location.id);
        if !remains.is_empty() {
            let names: Vec<String> = remains.iter().map(|corpse| corpse_label(corpse)).collect();
            println!("Remains here: {}", names.join(", "));
        }
        let exits: Vec<String> = location
            .exits
            .iter()
            .filter_map(|id| world.location_by_id(*id).map(|loc| loc.name.clone()))
            .collect();
        println!("Exits: {}", exits.join(", "));
    }
    if state.threat.active {
        println!("Threat: {}", state.threat.label);
        println!("{}", state.threat.description);
    }
    if !state.factions.is_empty() {
        let faction_lines: Vec<String> = state
            .factions
            .iter()
            .map(|faction| format!("{} ({:+})", faction.name, faction.reputation))
            .collect();
        println!("Factions: {}", faction_lines.join(", "));
    }
    if !state.quests.is_empty() {
        let quest_lines: Vec<String> = state
            .quests
            .iter()
            .map(|quest| {
                let status = if quest.completed {
                    if quest.reward_claimed { "completed" } else { "awaiting reward" }
                } else if quest.offered {
                    "active"
                } else {
                    "unheard"
                };
                format!("{} [{}]", quest.title, status)
            })
            .collect();
        println!("Quests: {}", quest_lines.join("; "));
    }
    println!("History entries: {}", world.history.len());
}

fn maybe_run_location_scene(state: &mut GameState) -> std::io::Result<()> {
    let location_id = state.character.location_id;
    if state.last_announced_location_id == Some(location_id) {
        return Ok(());
    }
    state.last_announced_location_id = Some(location_id);

    let mut lines = Vec::new();
    let npc_ids = npc_ids_at_location(state, location_id);
    for npc_id in npc_ids {
        lines.extend(location_scene_for_npc(state, npc_id, location_id));
    }

    if lines.is_empty() {
        return Ok(());
    }

    narrate(&lines.join("\n"));
    Ok(())
}

fn location_scene_for_npc(state: &mut GameState, npc_id: EntityId, location_id: EntityId) -> Vec<String> {
    let mut lines = Vec::new();
    let npc_index = match npc_index_by_id(state, npc_id) {
        Some(index) => index,
        None => return lines,
    };

    let npc_name = state.npcs[npc_index].display_name();
    let npc_title = state.npcs[npc_index].title.clone();
    let npc_location_name = state
        .world
        .location_by_id(state.npcs[npc_index].location_id)
        .map(|location| location.name.clone())
        .unwrap_or_else(|| "unknown place".to_string());
    let faction_id = state.npcs[npc_index].faction_id;
    let faction_name = faction_id.and_then(|id| faction_by_id(state, id).map(|faction| faction.name.clone()));
    let faction_rep = faction_id.and_then(|id| faction_by_id(state, id).map(|faction| faction.reputation)).unwrap_or(0);
    let is_market = npc_location_name == "Hollow Market";
    let quest = quest_by_title_mut(state, "Quiet the Old Shrine");

    if let Some(quest) = quest {
        if is_market && !quest.offered && matches!(faction_name.as_deref(), Some("Cinder Wardens")) {
            quest.offered = true;
            lines.push(format!(
                "{} says: 'The Old Shrine stirs again. Clear it, and the {} will remember your name.'",
                npc_name,
                faction_name.as_deref().unwrap_or("wardens")
            ));
            remember_npc(state, npc_id, "offered a shrine quest".to_string());
            if let Some(faction_id) = faction_id {
                remember_faction(state, faction_id, format!("{} offered a shrine quest at the market.", npc_name));
            }
        } else if quest.completed && !quest.reward_claimed && is_market {
            lines.push(format!("{} says: 'You did it. The shrine is quiet now.'", npc_name));
        } else if quest.completed && is_market {
            lines.push(format!("{} says: 'The shrine stays quiet because of you.'", npc_name));
        } else if faction_rep < 0 {
            lines.push(format!("{} watches you with open suspicion.", npc_name));
        } else if faction_rep > 0 {
            lines.push(format!("{} nods. 'You are not forgotten here.'", npc_name));
        } else {
            lines.push(format!("{} studies the road in silence.", npc_name));
        }
    } else if faction_rep > 0 {
        lines.push(format!("{} gives you a cautious nod.", npc_name));
    } else if faction_rep < 0 {
        lines.push(format!("{} looks away as you approach.", npc_name));
    }

    if state.threat.active && state.threat.source_location_id == Some(location_id) {
        lines.push(format!("{} glances at the threat and lowers their voice.", npc_name));
    }

    lines
}

fn remember_npc(state: &mut GameState, npc_id: EntityId, memory: String) {
    if let Some(index) = npc_index_by_id(state, npc_id) {
        let npc = &mut state.npcs[index];
        npc.memory.push(memory);
        if npc.memory.len() > 5 {
            let remove_count = npc.memory.len() - 5;
            npc.memory.drain(0..remove_count);
        }
    }
}

fn remember_faction(state: &mut GameState, faction_id: EntityId, memory: String) {
    if let Some(faction) = faction_by_id_mut(state, faction_id) {
        faction.memory.push(memory);
        if faction.memory.len() > 5 {
            let remove_count = faction.memory.len() - 5;
            faction.memory.drain(0..remove_count);
        }
    }
}

fn adjust_faction_reputation(state: &mut GameState, faction_id: EntityId, delta: i32, reason: &str) {
    if let Some(faction) = faction_by_id_mut(state, faction_id) {
        faction.reputation += delta;
        faction.memory.push(reason.to_string());
        if faction.memory.len() > 5 {
            let remove_count = faction.memory.len() - 5;
            faction.memory.drain(0..remove_count);
        }
    }
}

fn corpses_at_location<'a>(state: &'a GameState, location_id: EntityId) -> Vec<&'a Corpse> {
    state.corpses.iter().filter(|corpse| corpse.location_id == location_id).collect()
}

fn corpse_label(corpse: &Corpse) -> String {
    if corpse.former_name.is_empty() {
        "Unidentified remains".to_string()
    } else if corpse.scavenged {
        format!("{} the {} (searched)", corpse.former_name, corpse.former_title)
    } else {
        format!("{} the {}", corpse.former_name, corpse.former_title)
    }
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
            state.last_announced_location_id = None;
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
                    narrate("This place is dangerous.");
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
    narrate(&format!(
        "You settle your breathing, recover to {}/{}, and save the game.",
        state.character.hp,
        state.character.max_hp
    ));
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
            let enemy_name = encounter.enemy_name.clone();
            let character_name = state.character.display_name();
            state.threat.clear();
            if let Some(loc) = state.world.location_by_id_mut(location.id) {
                loc.dangerous = false;
            }
            state.character.turn += 1;
            state.world.record_history(state.character.turn, format!("{} defeated {} at {}.", character_name, enemy_name, location.name));
            let trophy = Item {
                id: encounter.enemy_id,
                name: format!("Trophy from {}", location.name),
                description: "A proof that the danger here was confronted and survived.".to_string(),
            };
            state.character.inventory.push(trophy.clone());
            notify_item_gain(&trophy);
            resolve_quest_for_location(state, location.id);
            update_faction_memory_for_location(state, location.id, format!("{} was cleared of danger.", location.name));
            narrate("The threat is broken. The place is quieter now.");
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
                    narrate("You brace yourself and hold the line.");
                }
            }
            Some(2) => {
                state.character.turn += 1;
                let character_name = state.character.display_name();
                state.world.record_history(
                    state.character.turn,
                    format!("{} fled from {} at {}.", character_name, encounter.enemy_name, location.name),
                );
                narrate("You back away and the threat remains.");
                break;
            }
            _ => {}
        }

        if state.character.hp <= 0 {
            let location_name = location.name.clone();
            mark_character_dead(state, format!("{} overcame them", encounter.enemy_name), &location_name);
            narrate("You were overwhelmed.");
            break;
        }
    }

    Ok(())
}

fn take_combat_damage(state: &mut GameState, damage: i32, enemy_name: &str, location_name: &str) {
    if damage <= 0 {
        narrate("The blow glances off harmlessly.");
        return;
    }

    state.character.hp -= damage;
    let character_name = state.character.display_name();
    state.world.record_history(
        state.character.turn,
        format!("{} took {} damage from {} at {}.", character_name, damage, enemy_name, location_name),
    );
    narrate(&format!("You take {} damage.", damage));
}

fn notify_item_gain(item: &Item) {
    println!("You gain: {}", item.name);
    println!("{}", item.description);
}

fn resolve_quest_for_location(state: &mut GameState, location_id: EntityId) {
    let quest_index = match state.quests.iter().position(|quest| quest.target_location_id == location_id) {
        Some(index) => index,
        None => return,
    };

    let faction_id;
    let quest_title;
    let completed_now;
    {
        let quest = &mut state.quests[quest_index];
        faction_id = quest.faction_id;
        quest_title = quest.title.clone();
        completed_now = !quest.completed;
        if completed_now {
            quest.completed = true;
        }
    }

    if !completed_now {
        return;
    }

    adjust_faction_reputation(state, faction_id, 10, &format!("{} completed.", quest_title));
    if let Some(faction) = faction_by_id(state, faction_id) {
        println!("{} reputation rises to {:+}.", faction.name, faction.reputation);
    }

    let reward = Item {
        id: state.world.allocate_id(),
        name: "Wardens' Seal".to_string(),
        description: "A rough token of trust from the wardens of the ash road.".to_string(),
    };
    state.character.inventory.push(reward.clone());
    notify_item_gain(&reward);

    if let Some(quest) = state.quests.get_mut(quest_index) {
        quest.reward_claimed = true;
    }

    let turn = state.character.turn;
    state.world.record_history(turn, format!("{} completed the quest: {}.", state.character.display_name(), quest_title));
    update_faction_memory_for_faction(state, faction_id, format!("{} completed the quest {}.", state.character.display_name(), quest_title));
    narrate("A faction contact will remember this.");
}

fn update_faction_memory_for_location(state: &mut GameState, location_id: EntityId, memory: String) {
    let npc_ids = npc_ids_at_location(state, location_id);
    let mut faction_ids = Vec::new();
    for npc_id in npc_ids {
        if let Some(index) = npc_index_by_id(state, npc_id) {
            let npc = &state.npcs[index];
            if let Some(faction_id) = npc.faction_id {
                faction_ids.push(faction_id);
                remember_npc(state, npc_id, memory.clone());
            }
        }
    }
    faction_ids.sort_unstable();
    faction_ids.dedup();
    for faction_id in faction_ids {
        remember_faction(state, faction_id, memory.clone());
    }
}

fn update_faction_memory_for_faction(state: &mut GameState, faction_id: EntityId, memory: String) {
    remember_faction(state, faction_id, memory.clone());
    let npc_ids: Vec<EntityId> = state
        .npcs
        .iter()
        .filter(|npc| npc.faction_id == Some(faction_id))
        .map(|npc| npc.id)
        .collect();
    for npc_id in npc_ids {
        remember_npc(state, npc_id, memory.clone());
    }
}

fn search_remains(state: &mut GameState) -> std::io::Result<()> {
    let location_id = state.character.location_id;
    let indices: Vec<usize> = state
        .corpses
        .iter()
        .enumerate()
        .filter(|(_, corpse)| corpse.location_id == location_id && !corpse.inventory.is_empty())
        .map(|(index, _)| index)
        .collect();

    if indices.is_empty() {
        println!("There are no remains worth searching here.");
        pause();
        return Ok(());
    }

    let options: Vec<String> = indices.iter().map(|index| corpse_label(&state.corpses[*index])).collect();
    if let Some(choice) = choose_from_list("Search which remains?", &options, Some("Back"))? {
        let corpse_index = indices[choice];
        let location_name = state
            .world
            .location_by_id(location_id)
            .map(|location| location.name.clone())
            .unwrap_or_else(|| "this place".to_string());

        let (former_name, former_title, items, corpse_id) = {
            let corpse = &mut state.corpses[corpse_index];
            let items = mem::take(&mut corpse.inventory);
            corpse.scavenged = true;
            (
                corpse.former_name.clone(),
                corpse.former_title.clone(),
                items,
                corpse.id,
            )
        };

        println!("You search the remains at {}.", location_name);
        if items.is_empty() {
            println!("Nothing useful remains.");
            state.world.record_history(
                state.character.turn,
                format!("{} searched the remains of {} the {} at {}.", state.character.display_name(), former_name, former_title, location_name),
            );
            pause();
            return Ok(());
        }

        for item in items {
            notify_item_gain(&item);
            state.character.inventory.push(item);
        }

        state.character.turn += 1;
        state.world.record_history(
            state.character.turn,
            format!("{} searched the remains of {} the {} at {}.", state.character.display_name(), former_name, former_title, location_name),
        );
        if let Some(location) = state.world.location_by_id_mut(location_id) {
            if !location.corpse_ids.contains(&corpse_id) {
                location.corpse_ids.push(corpse_id);
            }
        }
        narrate("You gather what can still be carried.");
    }

    Ok(())
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

fn review_quests(state: &GameState) {
    println!("\nQuest log for {}", state.character.display_name());
    if state.quests.is_empty() {
        println!("  Nothing yet.");
        pause();
        return;
    }

    for quest in &state.quests {
        let status = if quest.completed {
            if quest.reward_claimed { "completed" } else { "completed, reward pending" }
        } else if quest.offered {
            "active"
        } else {
            "unheard"
        };
        println!("  - {} [{}]", quest.title, status);
        println!("    {}", quest.description);
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
        narrate("The journal entry is recorded.");
    }
    Ok(())
}

fn force_death(state: &mut GameState) {
    state.character.hp = 0;
    let location_name = state
        .world
        .location_by_id(state.character.location_id)
        .map(|location| location.name.clone())
        .unwrap_or_else(|| "an unknown place".to_string());
    mark_character_dead(state, "a deliberate end".to_string(), &location_name);
    narrate("The character falls.");
}

fn mark_character_dead(state: &mut GameState, cause: String, location_name: &str) {
    if !state.character.alive {
        return;
    }

    state.character.alive = false;
    state.character.hp = 0;
    let corpse = create_corpse(state, cause.clone());
    let dropped_count = corpse.inventory.len();
    state.corpses.push(corpse.clone());
    if let Some(location) = state.world.location_by_id_mut(corpse.location_id) {
        if !location.corpse_ids.contains(&corpse.id) {
            location.corpse_ids.push(corpse.id);
        }
    }
    let character_name = state.character.display_name();
    state.world.record_history(
        state.character.turn,
        format!("{} died at {} ({cause}).", character_name, location_name),
    );
    update_faction_memory_for_location(state, corpse.location_id, format!("{} died at {}.", character_name, location_name));
    if dropped_count > 0 {
        println!("{} item(s) were left behind.", dropped_count);
    }
}

fn create_corpse(state: &mut GameState, epitaph: String) -> Corpse {
    let corpse_id = state.world.allocate_id();
    let location_id = state.character.location_id;
    let inventory = mem::take(&mut state.character.inventory);
    Corpse {
        id: corpse_id,
        former_name: state.character.name.clone(),
        former_title: state.character.title.clone(),
        location_id,
        turn_of_death: state.character.turn,
        inventory,
        epitaph,
        scavenged: false,
    }
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
