use crate::content::{load_campaign_content, CampaignContent};
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
    Talk,
    Meditate,
    QuestLog,
    Inventory,
    Journal,
    TestDeath,
    Quit,
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
    println!("The Ashen Chronicle v0.10.0");
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
    let report = crate::content::load_campaign_content_report();
    if !report.loaded_mods.is_empty() {
        println!("Loaded mods:");
        for manifest in &report.loaded_mods {
            let version = if manifest.version.is_empty() { "unknown" } else { manifest.version.as_str() };
            println!("  - {} ({}, v{})", manifest.name, manifest.id, version);
        }
    }
    if !report.warnings.is_empty() {
        eprintln!("Campaign content warnings:");
        for issue in &report.warnings {
            eprintln!("- {issue}");
        }
    }
    let content = &report.content;
    migrate_runtime_quest_data(state, content);
    ensure_campaign_factions(state, content);
    ensure_campaign_npcs(state, content);
    ensure_campaign_quests(state, content);
}

fn migrate_runtime_quest_data(state: &mut GameState, content: &CampaignContent) {
    for quest in &mut state.quests {
        if quest.content_id.is_empty() {
            if let Some(definition) = content
                .quests
                .iter()
                .find(|entry| entry.id == quest.title || entry.title == quest.title)
            {
                quest.content_id = definition.id.clone();
            }
        }
        if quest.reward_item_name.is_empty() {
            if let Some(definition) = content.quests.iter().find(|entry| entry.title == quest.title || entry.id == quest.content_id) {
                quest.reward_item_name = definition.reward_item_name.clone();
            }
        }
    }
}

fn ensure_campaign_factions(state: &mut GameState, content: &CampaignContent) {
    for faction in &content.factions {
        if faction_by_name(state, &faction.name).is_none() {
            let id = state.world.allocate_id();
            state.factions.push(Faction::new(id, faction.name.clone()));
        }
    }
}

fn ensure_campaign_npcs(state: &mut GameState, content: &CampaignContent) {
    for npc in &content.npcs {
        if npc_by_name(state, &npc.name).is_some() {
            continue;
        }
        let Some(location_id) = location_id_by_name(&state.world, &npc.location_name) else {
            continue;
        };
        let faction_id = npc
            .faction_name
            .as_deref()
            .and_then(|name| faction_id_by_name(state, name));
        let id = state.world.allocate_id();
        let mut runtime_npc = Npc::new(id, npc.name.clone(), npc.title.clone(), location_id, faction_id);
        for memory in &npc.memory {
            runtime_npc.memory.push(memory.clone());
        }
        state.npcs.push(runtime_npc);
    }
}

fn ensure_campaign_quests(state: &mut GameState, content: &CampaignContent) {
    migrate_completed_quest_deeds(state, content);
    for quest in &content.quests {
        ensure_quest(
            state,
            &quest.id,
            &quest.title,
            &quest.description,
            &quest.location_name,
            &quest.faction_name,
            &quest.giver_npc_name,
            &quest.required_item_name,
            &quest.reward_item_name,
        );
    }
}

fn migrate_completed_quest_deeds(state: &mut GameState, content: &CampaignContent) {
    let mut completed_ids = Vec::new();
    for quest in &state.quests {
        if !quest.completed {
            continue;
        }
        let content_id = quest_identity(quest, content);
        if !completed_ids.iter().any(|known| known == content_id) {
            completed_ids.push(content_id.to_string());
        }
    }

    for stored in state.world.completed_quest_ids.clone() {
        if !completed_ids.iter().any(|known| known == &stored) {
            if let Some(definition) = content.quests.iter().find(|entry| entry.title == stored || entry.id == stored) {
                completed_ids.push(definition.id.clone());
            } else {
                completed_ids.push(stored);
            }
        }
    }

    state.world.completed_quest_ids = completed_ids;
}

fn ensure_quest(
    state: &mut GameState,
    content_id: &str,
    title: &str,
    description: &str,
    location: &str,
    faction: &str,
    giver: &str,
    item: &str,
    reward_item_name: &str,
) {
    if state.quests.iter().any(|quest| quest.content_id == content_id || (quest.content_id.is_empty() && quest.title == title))
        || state.world.completed_quest_ids.iter().any(|known| known == content_id || known == title)
    {
        return;
    }
    if let (Some(location_id), Some(faction_id), Some(giver_npc_id)) = (
        location_id_by_name(&state.world, location), faction_id_by_name(state, faction), npc_id_by_name(state, giver)
    ) {
        let id = state.world.allocate_id();
        let mut quest = Quest::new(id, content_id.to_string(), title, description, location_id, faction_id, giver_npc_id, item, reward_item_name);
        // Content-driven quests begin unoffered and unfinished.
        quest.offered = false;
        state.quests.push(quest);
    }
}

fn quest_identity<'a>(quest: &'a Quest, content: &'a CampaignContent) -> &'a str {
    if !quest.content_id.is_empty() {
        &quest.content_id
    } else if let Some(definition) = content.quests.iter().find(|entry| entry.title == quest.title) {
        &definition.id
    } else {
        &quest.title
    }
}

fn quest_key<'a>(quest: &'a Quest) -> &'a str {
    if quest.content_id.is_empty() {
        &quest.title
    } else {
        &quest.content_id
    }
}


fn faction_by_name<'a>(state: &'a GameState, name: &str) -> Option<&'a Faction> {
    state.factions.iter().find(|faction| faction.name == name)
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

fn npc_id_by_name(state: &GameState, name: &str) -> Option<EntityId> {
    npc_by_name(state, name).map(|npc| npc.id)
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
                GameAction::Talk => talk(state)?,
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
        MenuEntry { label: "Talk".to_string(), action: GameAction::Talk },
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
    println!("History entries: {}", world.history.len());
}

fn maybe_run_location_scene(state: &mut GameState) -> std::io::Result<()> {
    let location_id = state.character.location_id;
    if state.last_announced_location_id == Some(location_id) {
        return Ok(());
    }
    state.last_announced_location_id = Some(location_id);

    let mut lines = location_atmosphere(state, location_id);
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

fn location_atmosphere(state: &GameState, location_id: EntityId) -> Vec<String> {
    let Some(location) = state.world.location_by_id(location_id) else { return Vec::new(); };
    let content = load_campaign_content();
    content
        .atmosphere_for(&location.name)
        .map(|text| vec![text.to_string()])
        .unwrap_or_default()
}

fn location_scene_for_npc(state: &mut GameState, npc_id: EntityId, location_id: EntityId) -> Vec<String> {
    let mut lines = Vec::new();
    let Some(npc_index) = npc_index_by_id(state, npc_id) else { return lines; };
    let npc_name = state.npcs[npc_index].display_name();

    if state.threat.active && state.threat.source_location_id == Some(location_id) {
        lines.push(format!("{} glances at the threat and lowers their voice.", npc_name));
    }

    lines
}

fn talk(state: &mut GameState) -> std::io::Result<()> {
    let location_id = state.character.location_id;
    let npc_ids = npc_ids_at_location(state, location_id);
    if npc_ids.is_empty() {
        println!("There is no one here to talk to.");
        pause();
        return Ok(());
    }

    let options: Vec<String> = npc_ids
        .iter()
        .filter_map(|id| npc_index_by_id(state, *id).map(|index| state.npcs[index].display_name()))
        .collect();
    if let Some(choice) = choose_from_list("Talk to whom?", &options, Some("Back"))? {
        talk_to_npc(state, npc_ids[choice])?;
    }
    Ok(())
}

fn talk_to_npc(state: &mut GameState, npc_id: EntityId) -> std::io::Result<()> {
    let Some(npc_index) = npc_index_by_id(state, npc_id) else { return Ok(()); };
    let npc_name = state.npcs[npc_index].display_name();
    let quest_indices: Vec<usize> = state
        .quests
        .iter()
        .enumerate()
        .filter(|(_, quest)| quest.giver_npc_id == npc_id)
        .map(|(index, _)| index)
        .collect();

    if quest_indices.is_empty() {
        println!("{} has little to say.", npc_name);
        pause();
        return Ok(());
    }

    let options = vec![
        "Ask if they need help".to_string(),
        "Tell them it's done".to_string(),
    ];
    if let Some(choice) = choose_from_list(&format!("Talk to {}", npc_name), &options, Some("Back"))? {
        match choice {
            0 => {
                let mut found_offer = false;
                for quest_index in quest_indices {
                    let (quest_key, title, description, faction_id, offered, completed) = {
                        let quest = &state.quests[quest_index];
                        (
                            quest_key(quest).to_string(),
                            quest.title.clone(),
                            quest.description.clone(),
                            quest.faction_id,
                            quest.offered,
                            quest.completed,
                        )
                    };
                    if state.world.completed_quest_ids.iter().any(|known| known == &quest_key) {
                        continue;
                    }
                    if completed {
                        continue;
                    }
                    found_offer = true;
                    if offered {
                        println!("{} says: 'You already agreed to help with {}.'", npc_name, title);
                    } else {
                        if let Some(quest) = state.quests.get_mut(quest_index) {
                            quest.offered = true;
                        }
                        println!("{} says: '{}'", npc_name, description);
                        remember_npc(state, npc_id, format!("offered the quest {}", title));
                        remember_faction(state, faction_id, format!("{} offered the quest {}.", npc_name, title));
                    }
                }
                if !found_offer {
                    println!("{} has no work for you. Whatever was asked here has already been done.", npc_name);
                }
                pause();
            }
            1 => {
                let mut handled = false;
                for quest_index in quest_indices {
                    // variable title is unused so changed it to _title
                    let (quest_key, _title, offered, completed, required_item_name) = {
                        let quest = &state.quests[quest_index];
                        (
                            quest_key(quest).to_string(),
                            quest.title.clone(),
                            quest.offered,
                            quest.completed,
                            quest.required_item_name.clone(),
                        )
                    };
                    if state.world.completed_quest_ids.iter().any(|known| known == &quest_key) || completed {
                        continue;
                    }
                    if !offered {
                        println!("{} does not know what you are talking about. You have not accepted any work from them.", npc_name);
                        handled = true;
                        continue;
                    }
                    handled = true;
                    if state.character.inventory.iter().any(|item| item.name == required_item_name) {
                        complete_quest(state, quest_index);
                    } else {
                        println!("{} looks at you expectantly. You have not brought the required proof.", npc_name);
                    }
                }
                if !handled {
                    println!("{} has no unfinished deed to hear about.", npc_name);
                }
                pause();
            }
            _ => {}
        }
    }
    Ok(())
}

fn complete_quest(state: &mut GameState, quest_index: usize) -> bool {
    let (quest_key, title, required_item_name, faction_id) = {
        let quest = &state.quests[quest_index];
        (
            quest_key(quest).to_string(),
            quest.title.clone(),
            quest.required_item_name.clone(),
            quest.faction_id,
        )
    };
    if state.world.completed_quest_ids.iter().any(|known| known == &quest_key) {
        return false;
    }

    let Some(item_index) = state.character.inventory.iter().position(|item| item.name == required_item_name) else {
        return false;
    };
    state.character.inventory.remove(item_index);

    let current_character_name = state.character.display_name();
    if let Some(quest) = state.quests.get_mut(quest_index) {
        quest.completed = true;
        quest.reward_claimed = true;
        quest.completed_by = Some(current_character_name.clone());
    }
    state.world.completed_quest_ids.push(quest_key.clone());

    // Reputation is split between doing the deed and carrying the faction's reward.
    adjust_faction_reputation(state, faction_id, 5, &format!("{} completed {}.", current_character_name, title));

    let reward_name = state
        .quests
        .get(quest_index)
        .map(|quest| quest.reward_item_name.clone())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Unnamed Reward".to_string());
    let reward = Item {
        id: state.world.allocate_id(),
        name: reward_name,
        description: format!("A token earned by completing {}.", title),
    };
    state.character.inventory.push(reward.clone());
    notify_item_gain(&reward);
    grant_reward_reputation(state, &reward);
    state.world.record_history(state.character.turn, format!("{} completed {}.", current_character_name, title));
    println!("The deed is recorded, and the quest item is no longer needed.");
    true
}

fn grant_reward_reputation(state: &mut GameState, item: &Item) {
    let Some(faction_name) = (match item.name.as_str() {
        "Wardens' Seal" => Some("Cinder Wardens"),
        "Rootworker's Token" => Some("Hollow Market Kin"),
        "Bell Covenant Charm" => Some("Drowned Bell Covenant"),
        _ => None,
    }) else { return; };
    let Some(faction_id) = faction_id_by_name(state, faction_name) else { return; };
    adjust_faction_reputation(state, faction_id, 5, &format!("Carrying {} marks affiliation with the faction.", item.name));
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

    let (enemy_name, enemy_hp, enemy_power, trophy_name) = encounter_profile(&location.name);
    let mut encounter = CombatEncounter { enemy_name, enemy_hp, enemy_power, enemy_id: state.world.allocate_id() };

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
                name: trophy_name.clone(),
                description: format!("A proof that the {} was confronted and survived.", location.name),
            };
            state.character.inventory.push(trophy.clone());
            notify_item_gain(&trophy);
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

fn encounter_profile(location_name: &str) -> (String, i32, i32, String) {
    let content = load_campaign_content();
    if let Some(profile) = content.encounter_for(location_name) {
        (
            profile.enemy_name.clone(),
            profile.enemy_hp,
            profile.enemy_power,
            profile.trophy_item_name.clone(),
        )
    } else {
        ("Ash-Crazed Marauder".to_string(), 7, 2, "Marauder's Token".to_string())
    }
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
            grant_reward_reputation(state, &item);
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
    println!();
    println!("Quest log for {}", state.character.display_name());
    let visible_quests: Vec<_> = state.quests.iter().filter(|quest| quest.offered || quest.completed).collect();
    if visible_quests.is_empty() {
        println!("  Nothing yet.");
        pause();
        return;
    }

    for quest in visible_quests {
        let status = if quest.completed {
            if quest.reward_claimed { "completed" } else { "completed, reward pending" }
        } else {
            "active"
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
