use crate::game::{actions, combat, screens};
use crate::model::GameState;
use std::io;
use std::path::Path;

pub(crate) fn dispatch(
    state: &mut GameState,
    action: actions::GameAction,
    save_path: &Path,
) -> io::Result<bool> {
    match action {
        actions::GameAction::Travel => actions::travel(state)?,
        actions::GameAction::InvestigateThreat => combat::investigate_threat(state)?,
        actions::GameAction::SearchRemains => actions::search_remains(state)?,
        actions::GameAction::Talk => actions::talk(state)?,
        actions::GameAction::Meditate => actions::meditate_and_save(state, save_path)?,
        actions::GameAction::QuestLog => actions::review_quests(state),
        actions::GameAction::Inventory => actions::show_inventory(state),
        actions::GameAction::Journal => actions::write_note(state)?,
        actions::GameAction::CharacterSheet => actions::character_sheet(state),
        actions::GameAction::TestDeath => actions::force_death(state),
        actions::GameAction::Quit => return screens::quit_screen(),
    }
    Ok(false)
}
