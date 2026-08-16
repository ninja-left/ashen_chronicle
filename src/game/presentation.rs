use crate::content::CampaignContent;
use crate::game::actions;
use crate::model::{EntityId, GameState};
use crate::ui::{set_dashboard, set_location_scene, Dashboard};
use std::io;

pub(crate) fn render_state(state: &GameState) {
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

pub(crate) fn maybe_run_location_scene(state: &mut GameState) -> io::Result<()> {
    let location_id = state.character.location_id;
    if state.last_announced_location_id == Some(location_id) {
        return Ok(());
    }
    state.last_announced_location_id = Some(location_id);
    let content = state
        .campaign_content
        .clone()
        .unwrap_or_else(crate::content::load_campaign_content);
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
