mod actions;
mod combat;
mod presentation;
mod runtime;
mod screens;
mod world;

use crate::model::GameState;

pub fn run() -> std::io::Result<()> {
    let _ui = crate::ui::init()?;
    let Some((mut state, mut save_path)) = screens::start_screen()? else {
        return Ok(());
    };
    world::bootstrap_campaign_content(&mut state);
    runtime::main_loop(&mut state, &mut save_path)
}

// Compatibility facade for screen code; validation itself belongs to the world module.
pub(crate) fn validate_loaded_state(state: &GameState) -> Vec<String> {
    world::validate_loaded_state(state)
}
