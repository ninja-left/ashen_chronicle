use crate::game::{actions, combat, presentation, screens, world};
use crate::model::GameState;
use crate::persistence::character_save_path;
use crate::ui::{choose_from_list, clear_log};
use std::io;
use std::path::PathBuf;

pub(crate) fn main_loop(state: &mut GameState, save_path: &mut PathBuf) -> io::Result<()> {
    loop {
        if !state.character.alive {
            clear_log();
            if !screens::death_screen(state)? {
                return Ok(());
            }
            *save_path = character_save_path(PathBuf::from(".").as_path(), &state.character.name);
            world::bootstrap_campaign_content(state);
            continue;
        }
        presentation::render_state(state);
        presentation::maybe_run_location_scene(state)?;
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
