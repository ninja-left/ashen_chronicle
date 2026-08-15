mod actions;
mod combat;
mod screens;

use crate::content::{load_campaign_content, CampaignContent};
use crate::model::{EntityId, Faction, GameState, Npc, Quest};
use crate::persistence::character_save_path;
use crate::ui::{choose_from_list, clear_log, set_dashboard, set_location_scene, Dashboard};
use std::path::PathBuf;

pub fn run() -> std::io::Result<()> {
    let _ui = crate::ui::init()?;
    let Some((mut state, mut save_path)) = screens::start_screen()? else {
        return Ok(());
    };
    bootstrap_campaign_content(&mut state);
    main_loop(&mut state, &mut save_path)
}

fn bootstrap_campaign_content(state: &mut GameState) {
    let content = state
        .campaign_content
        .clone()
        .unwrap_or_else(load_campaign_content);
    content.seed_world(&mut state.world);
    state.campaign_content = Some(content.clone());
    for faction_content in &content.factions {
        if state
            .factions
            .iter()
            .any(|faction| faction.name == faction_content.name)
        {
            continue;
        }
        let id = state.world.allocate_id();
        state
            .factions
            .push(Faction::new(id, faction_content.name.clone()));
    }
    for npc_content in &content.npcs {
        if state.npcs.iter().any(|npc| npc.name == npc_content.name) {
            continue;
        }
        let Some(location_id) = state
            .world
            .location_by_name(&npc_content.location_name)
            .map(|location| location.id)
        else {
            continue;
        };
        let faction_id = npc_content
            .faction_name
            .as_deref()
            .and_then(|name| actions::faction_id_by_name(state, name));
        let id = state.world.allocate_id();
        let mut npc = Npc::new(
            id,
            npc_content.name.clone(),
            npc_content.title.clone(),
            location_id,
            faction_id,
        );
        npc.memory = npc_content.memory.clone();
        state.npcs.push(npc);
    }
    for quest_content in &content.quests {
        if state
            .quests
            .iter()
            .any(|quest| quest.content_id == quest_content.id)
        {
            continue;
        }
        let Some(target_location_id) = state
            .world
            .location_by_name(&quest_content.location_name)
            .map(|location| location.id)
        else {
            continue;
        };
        let Some(faction_id) = actions::faction_id_by_name(state, &quest_content.faction_name)
        else {
            continue;
        };
        let Some(giver_npc_id) = state
            .npcs
            .iter()
            .find(|npc| npc.name == quest_content.giver_npc_name)
            .map(|npc| npc.id)
        else {
            continue;
        };
        let id = state.world.allocate_id();
        state.quests.push(Quest::new(
            id,
            quest_content.id.clone(),
            quest_content.title.clone(),
            quest_content.description.clone(),
            target_location_id,
            faction_id,
            giver_npc_id,
            quest_content.required_item_name.clone(),
            quest_content.reward_item_name.clone(),
        ));
    }
}

pub(crate) fn validate_loaded_state(state: &GameState) -> Vec<String> {
    let mut warnings = Vec::new();
    if !state.character.alive && state.character.hp > 0 {
        warnings.push("character is marked dead while still having HP".to_string());
    }
    if state
        .world
        .location_by_id(state.character.location_id)
        .is_none()
    {
        warnings.push(format!(
            "character references unknown location id {}",
            state.character.location_id
        ));
    }
    for npc in &state.npcs {
        if state.world.location_by_id(npc.location_id).is_none() {
            warnings.push(format!(
                "npc {} references unknown location id {}",
                npc.name, npc.location_id
            ));
        }
        if let Some(faction_id) = npc.faction_id {
            if !state
                .factions
                .iter()
                .any(|faction| faction.id == faction_id)
            {
                warnings.push(format!(
                    "npc {} references unknown faction id {}",
                    npc.name, faction_id
                ));
            }
        }
    }
    for quest in &state.quests {
        if state
            .world
            .location_by_id(quest.target_location_id)
            .is_none()
        {
            warnings.push(format!(
                "quest {} references unknown target location id {}",
                quest.title, quest.target_location_id
            ));
        }
        if !state
            .factions
            .iter()
            .any(|faction| faction.id == quest.faction_id)
        {
            warnings.push(format!(
                "quest {} references unknown faction id {}",
                quest.title, quest.faction_id
            ));
        }
        if !state.npcs.iter().any(|npc| npc.id == quest.giver_npc_id) {
            warnings.push(format!(
                "quest {} references unknown giver npc id {}",
                quest.title, quest.giver_npc_id
            ));
        }
    }
    warnings
}

fn main_loop(state: &mut GameState, save_path: &mut PathBuf) -> std::io::Result<()> {
    loop {
        if !state.character.alive {
            clear_log();
            if !screens::death_screen(state)? {
                return Ok(());
            }
            *save_path = character_save_path(PathBuf::from(".").as_path(), &state.character.name);
            bootstrap_campaign_content(state);
            continue;
        }
        render_state(state);
        maybe_run_location_scene(state)?;
        let menu = actions::build_main_menu(state);
        let labels: Vec<String> = menu.iter().map(|entry| entry.label.clone()).collect();
        let Some(choice) = choose_from_list("What will you do?", &labels, None)? else {
            continue;
        };
        clear_log();
        let result = match menu[choice].action {
            actions::GameAction::Travel => actions::travel(state),
            actions::GameAction::InvestigateThreat => combat::investigate_threat(state),
            actions::GameAction::SearchRemains => actions::search_remains(state),
            actions::GameAction::Talk => actions::talk(state),
            actions::GameAction::Meditate => actions::meditate_and_save(state, save_path),
            actions::GameAction::QuestLog => {
                actions::review_quests(state);
                Ok(())
            }
            actions::GameAction::Inventory => {
                actions::show_inventory(state);
                Ok(())
            }
            actions::GameAction::Journal => actions::write_note(state),
            actions::GameAction::CharacterSheet => {
                actions::character_sheet(state);
                Ok(())
            }
            actions::GameAction::TestDeath => {
                actions::force_death(state);
                Ok(())
            }
            actions::GameAction::Quit => {
                if screens::quit_screen()? {
                    return Ok(());
                }
                Ok(())
            }
        };
        result?;
    }
}

fn render_state(state: &GameState) {
    let world = &state.world;
    let character = &state.character;
    let location = world.location_by_id(character.location_id);
    let condition_line = if character.conditions.is_empty() {
        None
    } else {
        Some(format!(
            "Condition: {}",
            character
                .conditions
                .iter()
                .map(|c| format!("{} ({})", c.name, c.remaining))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    };
    let threat_line = if state.threat.active {
        Some(format!("Threat: {}", state.threat.label))
    } else {
        None
    };
    let dashboard = Dashboard {
        world_name: world.name.clone(),
        hp: character.hp,
        max_hp: character.max_hp,
        enemy_name: None,
        enemy_hp: None,
        enemy_max_hp: None,
        time_display: actions::time_display(world.time_points, world.day),
        condition_line,
        location_name: location.map(|location| format!("~ {} ~", location.name)),
        location_description: location.map(|location| location.description.clone()),
        danger_line: location.and_then(|location| {
            if location.dangerous {
                Some("You feel the danger.".to_string())
            } else {
                None
            }
        }),
        threat_line,
        action_hint: Some("Arrows / Enter / Esc".to_string()),
    };
    set_dashboard(dashboard);
}

fn maybe_run_location_scene(state: &mut GameState) -> std::io::Result<()> {
    let location_id = state.character.location_id;
    if state.last_announced_location_id == Some(location_id) {
        return Ok(());
    }
    state.last_announced_location_id = Some(location_id);
    let content = load_campaign_content();
    let mut lines = location_art(&content, state, location_id);
    let atmosphere = location_atmosphere(&content, state, location_id);
    let npc_ids = actions::npc_ids_at_location(state, location_id);
    if !lines.is_empty() && (!atmosphere.is_empty() || !npc_ids.is_empty()) {
        lines.push(String::new());
    }
    lines.extend(atmosphere);
    if !lines.is_empty() && !npc_ids.is_empty() {
        lines.push(String::new());
    }
    for npc_id in npc_ids {
        lines.extend(location_scene_for_npc(state, npc_id, location_id));
    }
    set_location_scene(lines);
    Ok(())
}

fn location_art(
    content: &CampaignContent,
    state: &GameState,
    location_id: EntityId,
) -> Vec<String> {
    let Some(location) = state.world.location_by_id(location_id) else {
        return Vec::new();
    };
    content
        .location_art_for(&location.name)
        .map(|art| art.lines().map(|line| line.to_string()).collect())
        .unwrap_or_default()
}

fn location_atmosphere(
    content: &CampaignContent,
    state: &GameState,
    location_id: EntityId,
) -> Vec<String> {
    let Some(location) = state.world.location_by_id(location_id) else {
        return Vec::new();
    };
    content
        .atmosphere_for(&location.name)
        .map(|text| vec![text.to_string()])
        .unwrap_or_default()
}

fn location_scene_for_npc(
    state: &mut GameState,
    npc_id: EntityId,
    location_id: EntityId,
) -> Vec<String> {
    let mut lines = Vec::new();
    let Some(npc_index) = actions::npc_index_by_id(state, npc_id) else {
        return lines;
    };
    let npc_name = state.npcs[npc_index].display_name();
    if state.threat.active && state.threat.source_location_id == Some(location_id) {
        lines.push(format!(
            "{} glances at the threat and lowers their voice.",
            npc_name
        ));
    }
    lines
}
