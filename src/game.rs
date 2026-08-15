use crate::content::{load_campaign_content, CampaignContent};
use crate::events::{trigger_event, EventContext};
use crate::model::{
    create_inherited_state, create_new_state, Condition, Corpse, EntityId, Faction, GameState,
    Item, Npc, Quest, WorldMode,
};
use crate::persistence::{
    character_save_path, find_save_files, legacy_save_path, load_game, save_game,
};
use crate::ui::{
    choose_from_list, clear_combat_health, clear_log, narrate, pause, prompt, set_combat_health,
    set_dashboard, set_location_scene, set_menu_screen, set_player_health, Dashboard,
};
use std::mem;
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

#[derive(Clone, Copy)]
enum GameAction {
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
    let _ui = crate::ui::init()?;
    let Some((mut state, mut save_path)) = start_screen()? else {
        return Ok(());
    };
    bootstrap_campaign_content(&mut state);
    main_loop(&mut state, &mut save_path)
}

fn start_screen() -> std::io::Result<Option<(GameState, PathBuf)>> {
    const VARIANTS: [(&str, &str); 4] = [
        (
            "The road is quiet. Something is listening.",
            r#"             .-.
            /   \
           /     \
      _____/       \_____
         \   /\   /
          \ /  \ /
           Y    Y
          /      \
         /        \
        /          \
       /            \
      /              \
     /                \
"#,
        ),
        (
            "The old gods are silent. The stones remember.",
            r#"             /\
            /  \
           /____\
          |      |
      _____|      |_____
          /        \
         /          \
        /            \
       /              \
      /________________\
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
             /   \
            /_____\

              ||
             /  \
            /____\
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
           /    .---.    \
          |    /     \    |
          |   |  o o  |   |
          |   |   ^   |   |
           \   \ '-' /   /
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
            .and_then(|name| faction_id_by_name(state, name));
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
        let Some(faction_id) = faction_id_by_name(state, &quest_content.faction_name) else {
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

fn validate_loaded_state(state: &GameState) -> Vec<String> {
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
            if !death_screen(state)? {
                return Ok(());
            }
            *save_path = character_save_path(PathBuf::from(".").as_path(), &state.character.name);
            bootstrap_campaign_content(state);
            continue;
        }

        render_state(state);
        maybe_run_location_scene(state)?;
        let menu = build_main_menu(state);
        let labels: Vec<String> = menu.iter().map(|entry| entry.label.clone()).collect();
        let Some(choice) = choose_from_list("What will you do?", &labels, None)? else {
            continue;
        };

        clear_log();
        let result = match menu[choice].action {
            GameAction::Travel => travel(state),
            GameAction::InvestigateThreat => investigate_threat(state),
            GameAction::SearchRemains => search_remains(state),
            GameAction::Talk => talk(state),
            GameAction::Meditate => meditate_and_save(state, save_path),
            GameAction::QuestLog => {
                review_quests(state);
                Ok(())
            }
            GameAction::Inventory => {
                show_inventory(state);
                Ok(())
            }
            GameAction::Journal => write_note(state),
            GameAction::CharacterSheet => {
                character_sheet(state);
                Ok(())
            }
            GameAction::TestDeath => {
                force_death(state);
                Ok(())
            }
            GameAction::Quit => {
                if quit_screen()? {
                    return Ok(());
                }
                Ok(())
            }
        };
        result?;
    }
}

fn npc_ids_at_location(state: &GameState, location_id: EntityId) -> Vec<EntityId> {
    state
        .npcs
        .iter()
        .filter(|npc| npc.location_id == location_id)
        .map(|npc| npc.id)
        .collect()
}

fn npc_index_by_id(state: &GameState, npc_id: EntityId) -> Option<usize> {
    state.npcs.iter().position(|npc| npc.id == npc_id)
}

fn quest_key(quest: &Quest) -> String {
    if quest.content_id.is_empty() {
        format!("legacy.quest.{}", quest.id)
    } else {
        quest.content_id.clone()
    }
}

fn faction_id_by_name(state: &GameState, faction_name: &str) -> Option<EntityId> {
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

fn quit_screen() -> std::io::Result<bool> {
    const VARIANTS: [(&str, &str, &str, &str); 9] = [
        (
            "The road ends here.\nFor tonight, anyway.",
            "Let the ashes take it.",
            "Not yet. The night has more to say.",
            r#"        .-''''-.
       /  .--.  \
      /  /    \  \
      | |      | |
      | |      | |
      |  \____/  |
       \        /
        '------'
"#,
        ),
        (
            "The fire is dying.\nYour story does not have to.",
            "Close the book.",
            "Turn the page.",
            r#"          /\
         /  \
        / /\ \
       / /  \ \
      /_/____\_\
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
    /           V           \
   /      _           _      \
   |     (_)         (_)     |
   |          .---.          |
    \        /     \        /
     '-._____'-----'_____.-'
"#,
        ),
        (
            "The last ember has gone black.\nThe silence is waiting.",
            "Let it be silent.",
            "Break the silence.",
            r#"            .
           / \
          /   \
         /_____\
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
       /|                    |\
      / |                    | \
     /  |                    |  \
    /   |                    |   \
   /    |                    |    \
  /_____|____________________|_____\
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
            r#"             /\
            /  \
           /____\
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
            r#"             /\                 /\
            /  \               /  \
           /    \             /    \
          /      \___________/      \
         /                         \
        /                           \
       /_____________________________\
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
    /     \  /     \  /     \
   /       \/       \/       \
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
          /             \
         /               \
        /                 \
       /                   \
      /                     \
     /_______________________\
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

fn build_main_menu(state: &GameState) -> Vec<MenuEntry> {
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
        time_display: time_display(world.time_points, world.day),
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
    let npc_ids = npc_ids_at_location(state, location_id);

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
    let Some(npc_index) = npc_index_by_id(state, npc_id) else {
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
                    {
                        continue;
                    }
                    if completed {
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

fn time_display(points: u32, day: u32) -> String {
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

fn advance_time(state: &mut GameState, amount: u32) {
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

fn add_or_refresh_condition(conditions: &mut Vec<Condition>, condition: Condition) {
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

fn gain_experience(state: &mut GameState, amount: u32) {
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

fn character_sheet(state: &GameState) {
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
            let context = EventContext::for_travel_arrival(
                &location_name,
                dangerous,
                is_night(state.world.time_points),
            );
            trigger_event(state, &context);

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

fn meditate_and_save(state: &mut GameState, save_path: &Path) -> std::io::Result<()> {
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
    narrate(&format!(
        "You meditate until your breathing steadies. You look at the sky...\n{}\nYou recover {} HP and save the game.",
        time_display(state.world.time_points, state.world.day), healing
    ));
    Ok(())
}

fn investigate_threat(state: &mut GameState) -> std::io::Result<()> {
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

    let (enemy_name, enemy_hp, enemy_power, trophy_name) = encounter_profile(state, &location.name);
    let enemy_max_hp = enemy_hp.max(1);
    let mut encounter = CombatEncounter {
        enemy_name,
        enemy_hp,
        enemy_power,
        enemy_id: state.world.allocate_id(),
    };

    set_player_health(state.character.hp, state.character.max_hp);
    set_combat_health(
        encounter.enemy_name.clone(),
        encounter.enemy_hp,
        enemy_max_hp,
    );
    println!("\nYou step into the threat.");

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
            state.world.record_history(
                state.character.turn,
                format!(
                    "{} defeated {} at {}.",
                    character_name, enemy_name, location.name
                ),
            );
            let trophy = Item {
                id: encounter.enemy_id,
                name: trophy_name.clone(),
                description: format!(
                    "A proof that the {} was confronted and survived.",
                    location.name
                ),
            };
            state.character.inventory.push(trophy.clone());
            notify_item_gain(state, &trophy);
            update_faction_memory_for_location(
                state,
                location.id,
                format!("{} was cleared of danger.", location.name),
            );
            gain_experience(state, 15);
            println!("\nCombat result: victory");
            println!("  Defeated: {}", enemy_name);
            println!("  Loot: {}", trophy.name);
            narrate("The threat is broken. The place is quieter now.");
            clear_combat_health();
            break;
        }

        set_combat_health(
            encounter.enemy_name.clone(),
            encounter.enemy_hp,
            enemy_max_hp,
        );
        let choices = vec![
            "Attack".to_string(),
            "Guard".to_string(),
            "Flee".to_string(),
        ];
        match choose_from_list("Combat action", &choices, None)? {
            Some(0) => {
                advance_time(state, 1);
                state.character.turn += 1;
                let damage = (3 + state.character.effective_might()).max(1);
                encounter.enemy_hp = (encounter.enemy_hp - damage).max(0);
                set_combat_health(
                    encounter.enemy_name.clone(),
                    encounter.enemy_hp,
                    enemy_max_hp,
                );
                println!("You strike {} for {} damage.", encounter.enemy_name, damage);
                let character_name = state.character.display_name();
                state.world.record_history(
                    state.character.turn,
                    format!(
                        "{} struck {} for {} damage.",
                        character_name, encounter.enemy_name, damage
                    ),
                );
                if encounter.enemy_hp > 0 {
                    let retaliation = encounter.enemy_power;
                    take_combat_damage(state, retaliation, &encounter.enemy_name, &location.name);
                }
            }
            Some(1) => {
                advance_time(state, 1);
                state.character.turn += 1;
                let retaliation =
                    (encounter.enemy_power - 1 - state.character.attributes.endurance / 2).max(0);
                let character_name = state.character.display_name();
                state.world.record_history(
                    state.character.turn,
                    format!(
                        "{} guarded against {}.",
                        character_name, encounter.enemy_name
                    ),
                );
                println!("You guard. Incoming damage is reduced to {}.", retaliation);
                if retaliation > 0 {
                    take_combat_damage(state, retaliation, &encounter.enemy_name, &location.name);
                }
            }
            Some(2) => {
                advance_time(state, 1);
                state.character.turn += 1;
                let character_name = state.character.display_name();
                state.world.record_history(
                    state.character.turn,
                    format!(
                        "{} fled from {} at {}.",
                        character_name, encounter.enemy_name, location.name
                    ),
                );
                println!("You flee. The threat remains.");
                pause();
                clear_combat_health();
                break;
            }
            _ => {}
        }

        if state.character.hp <= 0 {
            let location_name = location.name.clone();
            mark_character_dead(
                state,
                format!("{} overcame them", encounter.enemy_name),
                &location_name,
            );
            narrate("You were overwhelmed.");
            clear_combat_health();
            break;
        }
    }

    clear_combat_health();
    Ok(())
}

fn encounter_profile(state: &GameState, location_name: &str) -> (String, i32, i32, String) {
    if let Some(profile) = state
        .campaign_content
        .as_ref()
        .and_then(|content| content.encounter_for(location_name))
    {
        (
            profile.enemy_name.clone(),
            profile.enemy_hp,
            profile.enemy_power,
            profile.trophy_item_name.clone(),
        )
    } else {
        (
            "Ash-Crazed Marauder".to_string(),
            7,
            2,
            "Marauder's Token".to_string(),
        )
    }
}

fn take_combat_damage(state: &mut GameState, damage: i32, enemy_name: &str, location_name: &str) {
    if damage <= 0 {
        narrate("The blow glances off harmlessly.");
        return;
    }

    state.character.hp -= damage;
    set_player_health(state.character.hp, state.character.max_hp);
    let character_name = state.character.display_name();
    state.world.record_history(
        state.character.turn,
        format!(
            "{} took {} damage from {} at {}.",
            character_name, damage, enemy_name, location_name
        ),
    );
    println!("You take {} damage from {}.", damage, enemy_name);
}

fn notify_item_gain(state: &GameState, item: &Item) {
    println!("You gain: {}", item.name);
    println!("{}", item.description);
    print_item_visual(state, &item.name);
}

fn print_item_visual(state: &GameState, item_name: &str) {
    if let Some(art) = state
        .campaign_content
        .as_ref()
        .and_then(|content| content.item_art_for(item_name))
    {
        println!("");
        println!("{}", art);
    }
}

fn update_faction_memory_for_location(
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
                let hidden = Item {
                    id: state.world.allocate_id(),
                    name: "Ashen Note".to_string(),
                    description: "A scrap of writing that might reveal something about the life that ended here.".to_string(),
                };
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

fn write_note(state: &mut GameState) -> std::io::Result<()> {
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

fn death_screen(state: &mut GameState) -> std::io::Result<bool> {
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
          /         \
         /  _     _  \
        |  / \   / \  |
        |  \_/   \_/  |
         \     ^     /
          \_________/
             /|\
            / | \
           /  |  \
          /   |   \
         /    |    \
              |
             / \
            /   \
 "#,
        ),
        (
            "No crown survives the grave.",
            r#"          /\        /\
         /  \  /\  /  \
        /____\/  \/____\
             \  /
              \/
              ||
              ||
          ____||____
         /          \
        /____________\
"#,
        ),
        (
            "The earth remembers what you leave behind.",
            r#"              ______
             /      \
            /        \
           /          \
          /            \
         /______________\
              ||||
              ||||
              ||||
        ______||||______
       /                \
      /__________________\
"#,
        ),
        (
            "The body falls. The world does not.",
            r#"             /\
            /  \
           /    \
          /______\
             ||
        _____||_____
       /     ||     \
      /      ||      \
     /_______||_______\
             ||
            /  \
           /    \
          /______\
 "#,
        ),
        (
            "The flame gutters. The ash remains.",
            r#"              |
             / \
            /   \
           |     |
           |     |
           |_____|
             |||
             |||
          ___|||___
         /         \
        /___________\

             . . .
 "#,
        ),
        (
            "The body is still. The world is not.",
            r#"        .-''''-.
      .'  .--.  '.
     /   /    \   \
    |   |      |   |
    |   |      |   |
     \   \____/   /
      '.        .'
        '-.__.-'
"#,
        ),
        (
            "Your footprints end here. What you changed does not.",
            r#"          .-.
         /   \
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
