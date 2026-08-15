use super::actions;
use crate::model::{EntityId, GameState, Item};
use crate::ui::{
    choose_from_list, clear_combat_health, narrate, pause, set_combat_health, set_player_health,
};

macro_rules! println {
    () => {
        crate::ui::line("");
    };
    ($($arg:tt)*) => {
        crate::ui::line(&format!($($arg)*))
    };
}

struct CombatEncounter {
    enemy_name: String,
    enemy_hp: i32,
    enemy_power: i32,
    enemy_id: EntityId,
}

pub(crate) fn investigate_threat(state: &mut GameState) -> std::io::Result<()> {
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
            actions::update_faction_memory_for_location(
                state,
                location.id,
                format!("{} was cleared of danger.", location.name),
            );
            actions::gain_experience(state, 15);
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
                actions::advance_time(state, 1);
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
                actions::advance_time(state, 1);
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
                actions::advance_time(state, 1);
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
            actions::mark_character_dead(
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
