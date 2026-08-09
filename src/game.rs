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
    println!("The Ashen Chronicle v0.8.1");
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
    ensure_campaign_locations(state);
    ensure_campaign_factions(state);
    ensure_campaign_npcs(state);
    ensure_campaign_quests(state);
}

fn ensure_campaign_locations(state: &mut GameState) {
    let region_id = state.world.regions.first().map(|region| region.id).unwrap_or_else(|| {
        let id = state.world.allocate_id();
        state.world.regions.push(crate::model::Region {
            id,
            name: "The Ashen Crown".to_string(),
            description: "A bleak frontier where old stone roads still cut through soot and cinder.".to_string(),
            location_ids: Vec::new(),
        });
        id
    });

    let location_specs = [
        ("Charred Watchtower", "A leaning watchtower with a bell that rings when the wind changes.", false),
        ("Mourning Fields", "A field of ash where pale grass grows around old burial stones.", false),
        ("Blackroot Hollow", "A low ravine choked with black roots and the smell of wet iron.", true),
        ("Drowned Chapel", "A half-sunken chapel whose bell chamber disappears beneath dark water.", true),
        ("Sootbound Crossing", "A ruined road crossing where caravan tracks vanish into the cinder.", false),
    ];

    let mut added = Vec::new();
    for (name, description, dangerous) in location_specs {
        if location_id_by_name(&state.world, name).is_none() {
            let id = state.world.allocate_id();
            state.world.locations.push(crate::model::Location {
                id,
                name: name.to_string(),
                description: description.to_string(),
                region_id,
                dangerous,
                corpse_ids: Vec::new(),
                exits: Vec::new(),
            });
            added.push(id);
        }
    }

    let names = [
        "Ashen Gate", "Hollow Market", "Old Shrine", "Charred Watchtower",
        "Mourning Fields", "Blackroot Hollow", "Drowned Chapel", "Sootbound Crossing",
    ];
    let ids: Vec<EntityId> = names.iter().filter_map(|name| location_id_by_name(&state.world, name)).collect();
    if ids.len() == names.len() {
        let (gate, market, shrine, tower, fields, hollow, chapel, crossing) =
            (ids[0], ids[1], ids[2], ids[3], ids[4], ids[5], ids[6], ids[7]);
        set_exits(&mut state.world, gate, &[market, tower]);
        set_exits(&mut state.world, market, &[gate, shrine, crossing]);
        set_exits(&mut state.world, shrine, &[market, fields]);
        set_exits(&mut state.world, tower, &[gate, fields]);
        set_exits(&mut state.world, fields, &[shrine, tower, hollow]);
        set_exits(&mut state.world, hollow, &[fields, chapel]);
        set_exits(&mut state.world, chapel, &[hollow, crossing]);
        set_exits(&mut state.world, crossing, &[market, chapel]);
    }
    let region_location_ids: Vec<EntityId> = state.world.locations.iter().filter(|location| location.region_id == region_id).map(|location| location.id).collect();
    if let Some(region) = state.world.regions.iter_mut().find(|region| region.id == region_id) {
        region.location_ids = region_location_ids;
    }
    if !added.is_empty() {
        state.world.record_history(state.character.turn, "The old roads reveal forgotten places beyond the market and shrine.");
    }
}

fn set_exits(world: &mut crate::model::World, location_id: EntityId, exits: &[EntityId]) {
    if let Some(location) = world.location_by_id_mut(location_id) {
        location.exits = exits.to_vec();
    }
}

fn ensure_campaign_factions(state: &mut GameState) {
    if faction_by_name(state, "Cinder Wardens").is_none() {
        let id = state.world.allocate_id();
        state.factions.push(Faction::new(id, "Cinder Wardens"));
    }

    if faction_by_name(state, "Hollow Market Kin").is_none() {
        let id = state.world.allocate_id();
        state.factions.push(Faction::new(id, "Hollow Market Kin"));
    }
    if faction_by_name(state, "Drowned Bell Covenant").is_none() {
        let id = state.world.allocate_id();
        state.factions.push(Faction::new(id, "Drowned Bell Covenant"));
    }
}

fn ensure_campaign_npcs(state: &mut GameState) {
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
    let extra_npcs = [
        ("Ilyra", "Bell Keeper", "Drowned Chapel", "Drowned Bell Covenant", "Listens for bells beneath the water."),
        ("Tovin", "Grave Tender", "Mourning Fields", "Cinder Wardens", "Marks graves that the ash has not swallowed."),
        ("Kes", "Root Gatherer", "Blackroot Hollow", "Hollow Market Kin", "Trades medicines made from blackroot."),
    ];
    for (name, title, location_name, faction_name, memory) in extra_npcs {
        if npc_by_name(state, name).is_none() {
            if let (Some(location_id), Some(faction_id)) = (location_id_by_name(&state.world, location_name), faction_id_by_name(state, faction_name)) {
                let id = state.world.allocate_id();
                let mut npc = Npc::new(id, name, title, location_id, Some(faction_id));
                npc.memory.push(memory.to_string());
                state.npcs.push(npc);
            }
        }
    }
}

fn ensure_campaign_quests(state: &mut GameState) {
    migrate_completed_quest_deeds(state);
    ensure_quest(
        state,
        "Quiet the Old Shrine",
        "The wardens want the shrine cleared of whatever woke there.",
        "Old Shrine",
        "Cinder Wardens",
        "Mira",
        "Trophy from Old Shrine",
    );
    ensure_quest(
        state,
        "Roots for the Market",
        "Kes wants a fresh blackroot cutting from the hollow before the roots rot.",
        "Blackroot Hollow",
        "Hollow Market Kin",
        "Kes",
        "Rootbound Fang",
    );
    ensure_quest(
        state,
        "The Drowned Bell",
        "Ilyra asks you to recover the bell clapper from the drowned chapel.",
        "Drowned Chapel",
        "Drowned Bell Covenant",
        "Ilyra",
        "Drowned Rosary",
    );
}

fn migrate_completed_quest_deeds(state: &mut GameState) {
    let completed_titles: Vec<String> = state
        .quests
        .iter()
        .filter(|quest| quest.completed)
        .map(|quest| quest.title.clone())
        .collect();
    for title in completed_titles {
        if !state.world.completed_quest_titles.iter().any(|known| known == &title) {
            state.world.completed_quest_titles.push(title);
        }
    }
}

fn ensure_quest(state: &mut GameState, title: &str, description: &str, location: &str, faction: &str, giver: &str, item: &str) {
    if state.quests.iter().any(|quest| quest.title == title)
        || state.world.completed_quest_titles.iter().any(|known| known == title)
    {
        return;
    }
    if let (Some(location_id), Some(faction_id), Some(giver_npc_id)) = (
        location_id_by_name(&state.world, location), faction_id_by_name(state, faction), npc_id_by_name(state, giver)
    ) {
        let id = state.world.allocate_id();
        state.quests.push(Quest::new(id, title, description, location_id, faction_id, giver_npc_id, item));
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
    match location.name.as_str() {
        "Ashen Gate" => vec!["Wind slips through the broken towers, carrying the smell of cold iron.".to_string()],
        "Hollow Market" => vec!["A shutter moves by itself. Somewhere behind the empty stalls, coins clink once.".to_string()],
        "Old Shrine" => vec!["Ash gathers in the altar's cracks. Whatever stirred here has not forgotten the road.".to_string()],
        "Charred Watchtower" => vec!["The watchtower bell gives a single dull knock, though no hand touches it.".to_string()],
        "Mourning Fields" => vec!["Pale grass bends around old stones, exposing scraps of names beneath the ash.".to_string()],
        "Blackroot Hollow" => vec!["Black roots shift under the soil with a sound like distant breathing.".to_string()],
        "Drowned Chapel" => vec!["Water laps against the chapel steps. Far below, something answers with a bell note.".to_string()],
        "Sootbound Crossing" => vec!["Old wheel tracks divide at the crossing, then vanish where the ash has been disturbed.".to_string()],
        _ => Vec::new(),
    }
}

fn location_scene_for_npc(state: &mut GameState, npc_id: EntityId, location_id: EntityId) -> Vec<String> {
    let mut lines = Vec::new();
    let npc_index = match npc_index_by_id(state, npc_id) {
        Some(index) => index,
        None => return lines,
    };

    let npc_name = state.npcs[npc_index].display_name();
    let current_character_name = state.character.display_name();

    let quest_indices: Vec<usize> = state.quests.iter().enumerate().filter(|(_, quest)| quest.giver_npc_id == npc_id).map(|(index, _)| index).collect();
    for quest_index in quest_indices {
        let (offered, completed, required_item_name, title, description, quest_faction_id, completed_by) = {
            let quest = &state.quests[quest_index];
            (quest.offered, quest.completed, quest.required_item_name.clone(), quest.title.clone(), quest.description.clone(), quest.faction_id, quest.completed_by.clone())
        };
        if state.world.completed_quest_titles.iter().any(|known| known == &title) {
            continue;
        }
        let has_required_item = state.character.inventory.iter().any(|item| item.name == required_item_name);
        if !offered {
            if let Some(quest) = state.quests.get_mut(quest_index) { quest.offered = true; }
            lines.push(format!("{} says: '{}'", npc_name, description));
            remember_npc(state, npc_id, format!("offered the quest {}", title));
            remember_faction(state, quest_faction_id, format!("{} offered the quest {}.", npc_name, title));
        } else if !completed && has_required_item {
            if let Some(quest) = state.quests.get_mut(quest_index) {
                quest.completed = true;
                quest.reward_claimed = true;
                quest.completed_by = Some(current_character_name.clone());
            }
            if !state.world.completed_quest_titles.iter().any(|known| known == &title) {
                state.world.completed_quest_titles.push(title.clone());
            }
            if let Some(item_index) = state.character.inventory.iter().position(|item| item.name == required_item_name) {
                state.character.inventory.remove(item_index);
            }
            adjust_faction_reputation(state, quest_faction_id, 10, &format!("{} completed {}.", current_character_name, title));
            let reward_name = match title.as_str() {
                "Quiet the Old Shrine" => "Wardens' Seal",
                "Roots for the Market" => "Rootworker's Token",
                _ => "Bell Covenant Charm",
            };
            let reward = Item { id: state.world.allocate_id(), name: reward_name.to_string(), description: format!("A token earned by completing {}.", title) };
            state.character.inventory.push(reward.clone());
            notify_item_gain(&reward);
            state.world.record_history(state.character.turn, format!("{} completed {}.", current_character_name, title));
            lines.push(format!("{} accepts the proof and marks the deed in their memory.", npc_name));
        } else if completed {
            let pronoun = if completed_by.as_deref() == Some(current_character_name.as_str()) { "You have done this before." } else { "Another life carried this deed into history." };
            lines.push(format!("{} says: '{}'", npc_name, pronoun));
        }
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

    let (enemy_name, enemy_hp, enemy_power, trophy_name) = encounter_profile(&location.name);
    let mut encounter = CombatEncounter { enemy_name, enemy_hp, enemy_power, enemy_id: state.world.allocate_id() };
    let trophy_name = trophy_name.to_string();

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

fn encounter_profile(location_name: &str) -> (String, i32, i32, &str) {
    match location_name {
        "Old Shrine" => ("Ashen Wretch".to_string(), 6, 2, "Trophy from Old Shrine"),
        "Blackroot Hollow" => ("Rootbound Stalker".to_string(), 8, 2, "Rootbound Fang"),
        "Drowned Chapel" => ("Drowned Penitent".to_string(), 10, 3, "Drowned Rosary"),
        _ => ("Ash-Crazed Marauder".to_string(), 7, 2, "Marauder's Token"),
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
