mod actions;
mod combat;
mod presentation;
mod runtime;
mod screens;
mod world;

pub fn run() -> std::io::Result<()> {
    let _ui = crate::ui::init()?;
    let Some((mut state, mut save_path)) = screens::start_screen()? else {
        return Ok(());
    };
    world::bootstrap_campaign_content(&mut state);
    runtime::main_loop(&mut state, &mut save_path)
}
