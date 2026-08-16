use crate::game::{actions, dispatcher, presentation, screens, world};
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
        if dispatcher::dispatch(state, menu[choice].action, save_path)? {
            return Ok(());
        }
    }
}
