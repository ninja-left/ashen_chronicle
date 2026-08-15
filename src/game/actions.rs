use crate::model::{Condition, Corpse, EntityId, Faction, GameState, Item, Quest};
use crate::persistence::save_game;
use crate::ui::{choose_from_list, narrate, pause, prompt};
use std::mem;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

macro_rules! println {
    () => {
        crate::ui::line("");
    };
    ($($arg:tt)*) => {
        crate::ui::line(&format!($($arg)*))
    };
}

#[derive(Clone, Copy)]
pub(crate) enum GameAction {
    Travel,
    InvestigateThreat,
    SearchRemains,
    Talk,
    Meditate,
    QuestLog,
    Inventory,
    Journal,
    TestDeath,
    Quit,
    CharacterSheet,
}

pub(crate) struct MenuEntry {
    pub(crate) label: String,
    pub(crate) action: GameAction,
}

pub(crate) fn build_main_menu(state: &GameState) -> Vec<MenuEntry> {
    let mut menu = vec![
        MenuEntry {
            label: "Travel".to_string(),
            action: GameAction::Travel,
        },
        MenuEntry {
            label: "Meditate".to_string(),
            action: GameAction::Meditate,
        },
        MenuEntry {
            label: "Character sheet".to_string(),
            action: GameAction::CharacterSheet,
        },
        MenuEntry {
            label: "View inventory".to_string(),
            action: GameAction::Inventory,
        },
        MenuEntry {
            label: "Quest log".to_string(),
            action: GameAction::QuestLog,
        },
        MenuEntry {
            label: "Write journal note".to_string(),
            action: GameAction::Journal,
        },
        MenuEntry {
            label: "Talk".to_string(),
            action: GameAction::Talk,
        },
        MenuEntry {
            label: "Quit".to_string(),
            action: GameAction::Quit,
        },
        MenuEntry {
            label: "Test the death flow".to_string(),
            action: GameAction::TestDeath,
        },
    ];
    if state.threat.active {
        menu.insert(
            6,
            MenuEntry {
                label: "Investigate".to_string(),
                action: GameAction::InvestigateThreat,
            },
        );
    }
    if has_unscavenged_remains_at_location(state) {
        let insert_at = if state.threat.active { 7 } else { 6 };
        menu.insert(
            insert_at,
            MenuEntry {
                label: "Search remains".to_string(),
                action: GameAction::SearchRemains,
            },
        );
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

pub(crate) fn npc_ids_at_location(state: &GameState, location_id: EntityId) -> Vec<EntityId> {
    state
        .npcs
        .iter()
        .filter(|npc| npc.location_id == location_id)
        .map(|npc| npc.id)
        .collect()
}

pub(crate) fn npc_index_by_id(state: &GameState, npc_id: EntityId) -> Option<usize> {
    state.npcs.iter().position(|npc| npc.id == npc_id)
}

fn quest_key(quest: &Quest) -> String {
    if quest.content_id.is_empty() {
        format!("legacy.quest.{}", quest.id)
    } else {
        quest.content_id.clone()
    }
}

pub(crate) fn faction_id_by_name(state: &GameState, faction_name: &str) -> Option<EntityId> {
    state
        .factions
        .iter()
        .find(|faction| faction.name == faction_name)
        .map(|faction| faction.id)
}

fn faction_by_id_mut(state: &mut GameState, faction_id: EntityId) -> Option<&mut Faction> {
    state
        .factions
        .iter_mut()
        .find(|faction| faction.id == faction_id)
}

pub(crate) fn talk(state: &mut GameState) -> std::io::Result<()> {
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
    let Some(npc_index) = npc_index_by_id(state, npc_id) else {
        return Ok(());
    };
    let npc_name = state.npcs[npc_index].display_name();
    if !npc_is_available_now(state.world.time_points) {
        println!(
            "{}",
            npc_unavailable_message(&npc_name, state.world.time_points)
        );
        pause();
        return Ok(());
    }
    if let Some(memory) = state.npcs[npc_index].memory.last() {
        println!("{} remembers: {}", npc_name, memory);
    }
    if let Some(portrait) = state
        .campaign_content
        .as_ref()
        .and_then(|content| content.portrait_for(&state.npcs[npc_index].name))
    {
        println!("");
        println!("{}", portrait);
    }
    let quest_indices: Vec<usize> = state
        .quests
        .iter()
        .enumerate()
        .filter(|(_, quest)| quest.giver_npc_id == npc_id)
        .map(|(index, _)| index)
        .collect();
    let mut options = vec![
        "Ask if they need help".to_string(),
        "Tell them it's done".to_string(),
    ];
    let can_probe_memory =
        state.character.effective_insight() >= 2 && !state.npcs[npc_index].memory.is_empty();
    if can_probe_memory {
        options.push("Ask what they remember".to_string());
    }
    if quest_indices.is_empty() && !can_probe_memory {
        println!("{} has little to say.", npc_name);
        pause();
        return Ok(());
    }
    if let Some(choice) =
        choose_from_list(&format!("Talk to {}", npc_name), &options, Some("Back"))?
    {
        match choice {
            0 => {
                let mut found_offer = false;
                for quest_index in quest_indices {
                    let (quest_key, title, description, faction_id, offered, completed) = {
                        let quest = &state.quests[quest_index];
                        (
                            quest_key(quest),
                            quest.title.clone(),
                            quest.description.clone(),
                            quest.faction_id,
                            quest.offered,
                            quest.completed,
                        )
                    };
                    if state
                        .world
                        .completed_quest_ids
                        .iter()
                        .any(|known| known == &quest_key)
                        || completed
                    {
                        continue;
                    }
                    found_offer = true;
                    if offered {
                        println!(
                            "{} says: 'You already agreed to help with {}.'",
                            npc_name, title
                        );
                    } else {
                        if let Some(quest) = state.quests.get_mut(quest_index) {
                            quest.offered = true;
                        }
                        println!("{} says: '{}'", npc_name, description);
                        remember_npc(state, npc_id, format!("offered the quest {}", title));
                        remember_faction(
                            state,
                            faction_id,
                            format!("{} offered the quest {}.", npc_name, title),
                        );
                    }
                }
                if !found_offer {
                    println!(
                        "{} has no work for you. Whatever was asked here has already been done.",
                        npc_name
                    );
                }
                pause();
            }
            1 => {
                let mut handled = false;
                for quest_index in quest_indices {
                    let (quest_key, _title, offered, completed, required_item_name) = {
                        let quest = &state.quests[quest_index];
                        (
                            quest_key(quest),
                            quest.title.clone(),
                            quest.offered,
                            quest.completed,
                            quest.required_item_name.clone(),
                        )
                    };
                    if state
                        .world
                        .completed_quest_ids
                        .iter()
                        .any(|known| known == &quest_key)
                        || completed
                    {
                        continue;
                    }
                    if !offered {
                        println!("{} does not know what you are talking about. You have not accepted any work from them.", npc_name);
                        handled = true;
                        continue;
                    }
                    handled = true;
                    if state
                        .character
                        .inventory
                        .iter()
                        .any(|item| item.name == required_item_name)
                    {
                        complete_quest(state, quest_index);
                    } else {
                        println!(
                            "{} looks at you expectantly. You have not brought the required proof.",
                            npc_name
                        );
                    }
                }
                if !handled {
                    println!("{} has no unfinished deed to hear about.", npc_name);
                }
                pause();
            }
            2 if can_probe_memory => {
                if let Some(memory) = state.npcs[npc_index].memory.last() {
                    println!("{} searches your face, then recalls: {}", npc_name, memory);
                }
                pause();
            }
            _ => {}
        }
    }
    advance_time(state, 1);
    Ok(())
}

fn complete_quest(state: &mut GameState, quest_index: usize) -> bool {
    let (quest_key, title, required_item_name, faction_id) = {
        let quest = &state.quests[quest_index];
        (
            quest_key(quest),
            quest.title.clone(),
            quest.required_item_name.clone(),
            quest.faction_id,
        )
    };
    if state
        .world
        .completed_quest_ids
        .iter()
        .any(|known| known == &quest_key)
    {
        return false;
    }
    let Some(item_index) = state
        .character
        .inventory
        .iter()
        .position(|item| item.name == required_item_name)
    else {
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
    adjust_faction_reputation(
        state,
        faction_id,
        5,
        &format!("{} completed {}.", current_character_name, title),
    );
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
    notify_item_gain(state, &reward);
    grant_reward_reputation(state, &reward);
    state.world.record_history(
        state.character.turn,
        format!("{} completed {}.", current_character_name, title),
    );
    println!("\nQuest complete: {}", title);
    println!("  Quest item consumed: {}", required_item_name);
    println!("  Reward: {}", reward.name);
    gain_experience(state, 25);
    println!("  Reputation: +5 for completing the deed, +5 while carrying the reward");
    true
}

fn grant_reward_reputation(state: &mut GameState, item: &Item) {
    let Some(faction_name) = (match item.name.as_str() {
        "Wardens' Seal" => Some("Cinder Wardens"),
        "Rootworker's Token" => Some("Hollow Market Kin"),
        "Bell Covenant Charm" => Some("Drowned Bell Covenant"),
        _ => None,
    }) else {
        return;
    };
    let Some(faction_id) = faction_id_by_name(state, faction_name) else {
        return;
    };
    adjust_faction_reputation(
        state,
        faction_id,
        5,
        &format!("Carrying {} marks affiliation with the faction.", item.name),
    );
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

fn adjust_faction_reputation(
    state: &mut GameState,
    faction_id: EntityId,
    delta: i32,
    reason: &str,
) {
    if let Some(faction) = faction_by_id_mut(state, faction_id) {
        faction.reputation += delta;
        faction.memory.push(reason.to_string());
        if faction.memory.len() > 5 {
            let remove_count = faction.memory.len() - 5;
            faction.memory.drain(0..remove_count);
        }
    }
}

fn corpse_label(corpse: &Corpse) -> String {
    if corpse.former_name.is_empty() {
        "Unidentified remains".to_string()
    } else if corpse.scavenged {
        format!(
            "{} the {} (searched)",
            corpse.former_name, corpse.former_title
        )
    } else {
        format!("{} the {}", corpse.former_name, corpse.former_title)
    }
}

pub(crate) fn time_display(points: u32, day: u32) -> String {
    const PORTIONS: [&str; 12] = [
        "Deep Night",
        "Before Dawn",
        "Dawn",
        "Morning",
        "Late Morning",
        "High Sun",
        "Afternoon",
        "Late Afternoon",
        "Dusk",
        "Evening",
        "Night",
        "Midnight",
    ];
    const WIDTH: usize = 23;
    let slot = (points % 12) as usize;
    let label = PORTIONS[slot];
    let mut top = vec![' '; WIDTH];
    let mut bottom = vec![' '; WIDTH];
    let place = |line: &mut Vec<char>, idx: usize, ch: char| {
        if idx < line.len() {
            line[idx] = ch;
        }
    };
    match slot {
        0 => place(&mut bottom, 20, '☾'),
        1 => place(&mut bottom, 16, '☾'),
        2 => place(&mut top, 16, '○'),
        3 => place(&mut top, 13, '○'),
        4 => place(&mut top, 10, '○'),
        5 => place(&mut top, 7, '○'),
        6 => place(&mut top, 4, '○'),
        7 => place(&mut bottom, 4, '○'),
        8 => place(&mut bottom, 7, '☾'),
        9 => place(&mut bottom, 10, '☾'),
        10 => place(&mut bottom, 13, '☾'),
        11 => place(&mut bottom, 16, '☾'),
        _ => unreachable!(),
    }
    let top: String = top.into_iter().collect();
    let bottom: String = bottom.into_iter().collect();
    let indicator = format!("E{}W", "=".repeat(WIDTH - 2));
    format!(
        "{}\n{}\n{}  Day {} | {}",
        top, bottom, indicator, day, label
    )
}

fn npc_unavailable_message(npc_name: &str, points: u32) -> String {
    let slot = points % 12;
    let (reason, hint) = match slot {
        0 | 1 => ("It is too late in the night.", "Try again after dawn."),
        10 | 11 => ("It is too late tonight.", "Try again in the morning."),
        2..=5 => (
            "It is still too early in the day.",
            "Try again later today.",
        ),
        6..=9 => ("It is too late in the day.", "Try again tomorrow morning."),
        _ => ("They are unavailable right now.", "Try again later."),
    };
    format!(
        "{} is not available right now. {} {}",
        npc_name, reason, hint
    )
}

pub(crate) fn advance_time(state: &mut GameState, amount: u32) {
    let total = state.world.time_points + amount;
    state.world.day += total / 12;
    state.world.time_points = total % 12;
    for condition in &mut state.character.conditions {
        condition.remaining = condition.remaining.saturating_sub(amount);
    }
    state
        .character
        .conditions
        .retain(|condition| condition.remaining > 0);
    if amount > 0 && state.character.hp <= state.character.max_hp / 3 && state.character.alive {
        add_or_refresh_condition(
            &mut state.character.conditions,
            Condition::new("Wounded", 3, -1),
        );
    }
}

pub(crate) fn add_or_refresh_condition(conditions: &mut Vec<Condition>, condition: Condition) {
    if let Some(existing) = conditions
        .iter_mut()
        .find(|current| current.name == condition.name)
    {
        existing.remaining = existing.remaining.max(condition.remaining);
        existing.penalty = condition.penalty;
        existing.bonus = condition.bonus;
    } else {
        conditions.push(condition);
    }
}

fn remove_condition(conditions: &mut Vec<Condition>, name: &str) {
    conditions.retain(|condition| condition.name != name);
}
fn is_night(points: u32) -> bool {
    matches!(points % 12, 0 | 1 | 10 | 11)
}
fn npc_is_available_now(points: u32) -> bool {
    matches!(points % 12, 2..=9)
}

pub(crate) fn gain_experience(state: &mut GameState, amount: u32) {
    state.character.experience += amount;
    loop {
        let threshold = state.character.level * 50;
        if state.character.experience < threshold {
            break;
        }
        state.character.experience -= threshold;
        state.character.level += 1;
        println!(
            "\nYou have grown stronger. You reached level {}.",
            state.character.level
        );
        let options = vec![
            "Might (+1 attack)".to_string(),
            "Insight (+1 search/recovery)".to_string(),
            "Endurance (+1 meditation healing)".to_string(),
        ];
        if let Ok(Some(choice)) = choose_from_list("Choose a new strength", &options, None) {
            match choice {
                0 => state.character.attributes.might += 1,
                1 => state.character.attributes.insight += 1,
                _ => state.character.attributes.endurance += 1,
            }
        }
    }
}

pub(crate) fn character_sheet(state: &GameState) {
    println!("\n=== Character ===");
    println!("{}", state.character.display_name());
    println!(
        "Level {}  XP {}/{}",
        state.character.level,
        state.character.experience,
        state.character.level * 50
    );
    println!(
        "Might: {}  Insight: {}  Endurance: {}",
        state.character.attributes.might,
        state.character.attributes.insight,
        state.character.attributes.endurance
    );
    println!(
        "Effective might: {}  Effective insight: {}",
        state.character.effective_might(),
        state.character.effective_insight()
    );
    if !state.character.conditions.is_empty() {
        println!(
            "Conditions: {}",
            state
                .character
                .conditions
                .iter()
                .map(|c| format!("{} ({} portions)", c.name, c.remaining))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    if state.factions.is_empty() {
        println!("Faction reputation: none");
    } else {
        println!("Faction reputation:");
        for faction in &state.factions {
            println!("  - {} {:+}", faction.name, faction.reputation);
        }
    }
    pause();
}

pub(crate) fn travel(state: &mut GameState) -> std::io::Result<()> {
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
            advance_time(state, 2);
            if is_night(state.world.time_points) {
                add_or_refresh_condition(
                    &mut state.character.conditions,
                    Condition::new("Exhausted", 2, -1),
                );
            }
            state.character.turn += 1;
            state.character.location_id = target_id;
            state.threat.clear();
            state.last_announced_location_id = None;
            let location = state.world.location_by_id(target_id).cloned();
            let location_name = location
                .as_ref()
                .map(|loc| loc.name.clone())
                .unwrap_or_else(|| "Unknown".to_string());
            let character_name = state.character.display_name();
            state.world.record_history(
                state.character.turn,
                format!("{} traveled to {}.", character_name, location_name),
            );
            println!("You travel to {}.", location_name);
            let dangerous = location.as_ref().map(|loc| loc.dangerous).unwrap_or(false);
            let context = crate::events::EventContext::for_travel_arrival(
                &location_name,
                dangerous,
                is_night(state.world.time_points),
            );
            crate::events::trigger_event(state, &context);
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

pub(crate) fn meditate_and_save(state: &mut GameState, save_path: &Path) -> std::io::Result<()> {
    let location_is_dangerous = state
        .world
        .location_is_dangerous(state.character.location_id);
    if state.threat.active || location_is_dangerous {
        println!("Not safe enough to meditate here.");
        pause();
        return Ok(());
    }
    let input = prompt("How long will you meditate? [1-4 time portions] ")?;
    let portions = input
        .parse::<u32>()
        .ok()
        .filter(|value| (1..=4).contains(value))
        .unwrap_or(1);
    let healing = portions as i32 + state.character.effective_endurance();
    advance_time(state, portions);
    state.character.turn += 1;
    state.character.heal(healing);
    remove_condition(&mut state.character.conditions, "Exhausted");
    let mut rested = Condition::new("Well-rested", 3, 0);
    rested.bonus = 1;
    add_or_refresh_condition(&mut state.character.conditions, rested);
    let character_name = state.character.display_name();
    state.world.record_history(
        state.character.turn,
        format!(
            "{} meditated for {} time portions and recovered.",
            character_name, portions
        ),
    );
    save_game(save_path, state)?;
    narrate(&format!("You meditate until your breathing steadies. You look at the sky...\n{}\nYou recover {} HP and save the game.", time_display(state.world.time_points, state.world.day), healing));
    Ok(())
}

pub(crate) fn search_remains(state: &mut GameState) -> std::io::Result<()> {
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
    let options: Vec<String> = indices
        .iter()
        .map(|index| corpse_label(&state.corpses[*index]))
        .collect();
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
        advance_time(state, 1);
        println!("You search the remains at {}.", location_name);
        if items.is_empty() {
            println!("Nothing useful remains.");
            state.world.record_history(
                state.character.turn,
                format!(
                    "{} searched the remains of {} the {} at {}.",
                    state.character.display_name(),
                    former_name,
                    former_title,
                    location_name
                ),
            );
            pause();
            return Ok(());
        }
        let item_names: Vec<String> = items.iter().map(|item| item.name.clone()).collect();
        for item in items {
            notify_item_gain(state, &item);
            grant_reward_reputation(state, &item);
            state.character.inventory.push(item);
        }
        if state.character.effective_insight() >= 2 && item_names.len() < 3 {
            let tick = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0);
            if tick.is_multiple_of(2) {
                let hidden = Item { id: state.world.allocate_id(), name: "Ashen Note".to_string(), description: "A scrap of writing that might reveal something about the life that ended here.".to_string() };
                notify_item_gain(state, &hidden);
                state.character.inventory.push(hidden);
                println!("Your insight uncovers something the hurried would have missed.");
            }
        }
        gain_experience(
            state,
            (5 + state.character.effective_insight())
                .try_into()
                .unwrap(),
        );
        println!("Feel like a deja-vu.");
        println!("You feel as if they were once yours. Though, These items can be inherited, Their memories cannot.");
        println!("Recovered {}", item_names.join(", "));
        state.character.turn += 1;
        state.world.record_history(
            state.character.turn,
            format!(
                "{} searched the remains of {} the {} at {}.",
                state.character.display_name(),
                former_name,
                former_title,
                location_name
            ),
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

fn notify_item_gain(state: &GameState, item: &Item) {
    println!("You gain: {}", item.name);
    println!("{}", item.description);
    if let Some(art) = state
        .campaign_content
        .as_ref()
        .and_then(|content| content.item_art_for(&item.name))
    {
        println!("");
        println!("{}", art);
    }
}

pub(crate) fn show_inventory(state: &GameState) {
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

pub(crate) fn review_quests(state: &GameState) {
    println!();
    println!("Quest log for {}", state.character.display_name());
    let visible_quests: Vec<_> = state
        .quests
        .iter()
        .filter(|quest| quest.offered || quest.completed)
        .collect();
    if visible_quests.is_empty() {
        println!("  Nothing yet.");
        pause();
        return;
    }
    for quest in visible_quests {
        let status = if quest.completed {
            if quest.reward_claimed {
                "completed"
            } else {
                "completed, reward pending"
            }
        } else {
            "active"
        };
        println!("  - {} [{}]", quest.title, status);
        println!("    {}", quest.description);
    }
    pause();
}

pub(crate) fn write_note(state: &mut GameState) -> std::io::Result<()> {
    let note = prompt("Write a journal note: ")?;
    if !note.is_empty() {
        state.character.notes.push(note.clone());
        advance_time(state, 1);
        state.character.turn += 1;
        let character_name = state.character.display_name();
        state.world.record_history(
            state.character.turn,
            format!("{} noted: {}", character_name, note),
        );
        narrate("The journal entry is recorded.");
    }
    Ok(())
}

pub(crate) fn force_death(state: &mut GameState) {
    state.character.hp = 0;
    let location_name = state
        .world
        .location_by_id(state.character.location_id)
        .map(|location| location.name.clone())
        .unwrap_or_else(|| "an unknown place".to_string());
    mark_character_dead(state, "a deliberate end".to_string(), &location_name);
    narrate("The character falls.");
}

pub(crate) fn mark_character_dead(state: &mut GameState, cause: String, location_name: &str) {
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
    update_faction_memory_for_location(
        state,
        corpse.location_id,
        format!("{} died at {}.", character_name, location_name),
    );
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

pub(crate) fn update_faction_memory_for_location(
    state: &mut GameState,
    location_id: EntityId,
    memory: String,
) {
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
