use crate::content::{EventConditionContent, EventContent, EventEffectContent};
use crate::model::{Condition, GameState};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct EventContext<'a> {
    pub trigger: &'a str,
    pub location_name: &'a str,
    pub dangerous: bool,
    pub night: bool,
}

impl<'a> EventContext<'a> {
    pub fn for_travel_arrival(location_name: &'a str, dangerous: bool, night: bool) -> Self {
        Self { trigger: "travel_arrival", location_name, dangerous, night }
    }
}

pub fn trigger_event(state: &mut GameState, context: &EventContext<'_>) -> bool {
    let Some(chosen) = ({
        let Some(content) = state.campaign_content.as_ref() else {
            return false;
        };
        let chance_roll = random_roll() % 100;
        let candidates: Vec<&EventContent> = content
            .events
            .iter()
            .filter(|event| event.trigger == context.trigger)
            .filter(|event| matches_conditions(event.conditions.as_ref(), state, context))
            .filter(|event| event_is_off_cooldown(state, &event.id))
            .filter(|event| event.chance_percent.unwrap_or(100) as u64 > chance_roll)
            .collect();
        if candidates.is_empty() {
            None
        } else {
            Some(weighted_pick(&candidates, random_roll()).unwrap_or(candidates[0]).clone())
        }
    }) else {
        return false;
    };
    apply_event(state, &chosen, context);
    true
}

fn matches_conditions(
    conditions: Option<&EventConditionContent>,
    state: &GameState,
    context: &EventContext<'_>,
) -> bool {
    let Some(conditions) = conditions else { return true; };

    if let Some(night) = conditions.night {
        if context.night != night { return false; }
    }
    if let Some(dangerous) = conditions.dangerous {
        if context.dangerous != dangerous { return false; }
    }
    if let Some(min_day) = conditions.min_day {
        if state.world.day < min_day { return false; }
    }
    if let Some(max_day) = conditions.max_day {
        if state.world.day > max_day { return false; }
    }
    if !conditions.locations.is_empty()
        && !conditions.locations.iter().any(|location| location == context.location_name)
    {
        return false;
    }
    if let Some(prior_event_id) = conditions.prior_event_id.as_deref() {
        if !state.world.history.iter().any(|entry| entry.event_id.as_deref() == Some(prior_event_id)) {
            return false;
        }
    }
    if conditions.min_reputation.is_some() || conditions.max_reputation.is_some() {
        let Some(faction_name) = conditions.faction_name.as_deref() else {
            return false;
        };
        let reputation = state
            .factions
            .iter()
            .find(|faction| faction.name == faction_name)
            .map(|faction| faction.reputation)
            .unwrap_or(0);
        if let Some(min_reputation) = conditions.min_reputation {
            if reputation < min_reputation {
                return false;
            }
        }
        if let Some(max_reputation) = conditions.max_reputation {
            if reputation > max_reputation {
                return false;
            }
        }
    }
    if let Some(item_name) = conditions.required_item_name.as_deref() {
        if !state.character.inventory.iter().any(|item| item.name == item_name) {
            return false;
        }
    }
    if let Some(condition_name) = conditions.required_condition_name.as_deref() {
        if !state.character.conditions.iter().any(|condition| condition.name == condition_name) {
            return false;
        }
    }
    true
}

fn event_is_off_cooldown(state: &GameState, event_id: &str) -> bool {
    state
        .world
        .event_cooldowns
        .iter()
        .find(|cooldown| cooldown.event_id == event_id)
        .map(|cooldown| state.character.turn >= cooldown.ready_at_turn)
        .unwrap_or(true)
}

fn weighted_pick<'a>(events: &[&'a EventContent], roll: u64) -> Option<&'a EventContent> {
    let total_weight: u64 = events.iter().map(|event| event.weight.max(1) as u64).sum();
    if total_weight == 0 { return None; }
    let mut cursor = roll % total_weight;
    for event in events {
        let weight = event.weight.max(1) as u64;
        if cursor < weight { return Some(event); }
        cursor -= weight;
    }
    events.last().copied()
}

fn random_roll() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0)
}

fn apply_event(state: &mut GameState, event: &EventContent, context: &EventContext<'_>) {
    let mut outcomes = Vec::new();
    for effect in &event.effects {
        apply_effect(state, effect, context, &mut outcomes);
    }

    let outcome = if outcomes.is_empty() {
        "The event leaves its mark.".to_string()
    } else {
        outcomes.join(" ")
    };
    state.world.record_event_history(
        state.character.turn,
        event.id.clone(),
        context.location_name.to_string(),
        outcome,
    );

    let cooldown_turns = event.cooldown_turns.unwrap_or(0);
    if cooldown_turns > 0 {
        let ready_at_turn = state.character.turn.saturating_add(cooldown_turns.saturating_add(1));
        if let Some(cooldown) = state.world.event_cooldowns.iter_mut().find(|entry| entry.event_id == event.id) {
            cooldown.ready_at_turn = ready_at_turn;
        } else {
            state.world.event_cooldowns.push(crate::model::EventCooldown {
                event_id: event.id.clone(),
                ready_at_turn,
            });
        }
    }
}

fn apply_effect(
    state: &mut GameState,
    effect: &EventEffectContent,
    context: &EventContext<'_>,
    outcomes: &mut Vec<String>,
) {
    match effect {
        EventEffectContent::Message { text } => {
            let rendered = render_text(text, state, context);
            crate::ui::line(&rendered);
            outcomes.push(rendered);
        }
        EventEffectContent::History { text } => {
            let rendered = render_text(text, state, context);
            state.world.record_history(state.character.turn, rendered.clone());
            outcomes.push(rendered);
        }
        EventEffectContent::Pause => crate::ui::pause(),
        EventEffectContent::Heal { amount } => {
            state.character.heal(*amount);
            outcomes.push(format!("Recovered {} HP.", amount));
        }
        EventEffectContent::Damage { amount } => {
            state.character.hp = (state.character.hp - amount).max(0);
            if state.character.hp == 0 { state.character.alive = false; }
            outcomes.push(format!("Suffered {} damage.", amount));
        }
        EventEffectContent::AddCondition { name, remaining, penalty, bonus } => {
            let mut condition = Condition::new(name.clone(), *remaining, *penalty);
            condition.bonus = *bonus;
            state.character.conditions.push(condition);
            outcomes.push(format!("Gained {}.", name));
        }
    }
}

fn render_text(text: &str, state: &GameState, context: &EventContext<'_>) -> String {
    text.replace("{character}", &state.character.display_name())
        .replace("{location}", context.location_name)
        .replace("{day}", &state.world.day.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::EventConditionContent;
    use crate::model::{create_new_state, WorldMode};

    fn test_state() -> GameState {
        create_new_state("Tester", WorldMode::New, "Ash Walker".into(), "the Test Subject".into())
    }

    #[test]
    fn night_condition_matches_expected_context() {
        let state = test_state();
        let condition = EventConditionContent { night: Some(true), ..Default::default() };
        let day = EventContext::for_travel_arrival("Ashen Gate", false, false);
        let night = EventContext::for_travel_arrival("Ashen Gate", false, true);
        assert!(!matches_conditions(Some(&condition), &state, &day));
        assert!(matches_conditions(Some(&condition), &state, &night));
    }

    #[test]
    fn weighted_pick_respects_weight_boundaries() {
        let first = EventContent { id: "first".into(), trigger: "test".into(), weight: 1, chance_percent: Some(100), cooldown_turns: None, conditions: None, effects: vec![] };
        let second = EventContent { id: "second".into(), trigger: "test".into(), weight: 3, chance_percent: Some(100), cooldown_turns: None, conditions: None, effects: vec![] };
        let events = vec![&first, &second];
        assert_eq!(weighted_pick(&events, 0).unwrap().id, "first");
        assert_eq!(weighted_pick(&events, 1).unwrap().id, "second");
        assert_eq!(weighted_pick(&events, 3).unwrap().id, "second");

    }

    #[test]
    fn reputation_condition_matches_current_faction_standing() {
        let mut state = test_state();
        state.factions.push(crate::model::Faction::new(99, "Cinder Wardens"));
        state.factions[0].reputation = 5;
        let context = EventContext::for_travel_arrival("Ashen Gate", false, false);
        let condition = EventConditionContent {
            faction_name: Some("Cinder Wardens".into()),
            min_reputation: Some(5),
            max_reputation: Some(10),
            ..Default::default()
        };
        assert!(matches_conditions(Some(&condition), &state, &context));
        state.factions[0].reputation = 4;
        assert!(!matches_conditions(Some(&condition), &state, &context));
    }

    #[test]
    fn inventory_and_condition_requirements_are_enforced() {
        let mut state = test_state();
        let context = EventContext::for_travel_arrival("Ashen Gate", false, false);
        let condition = EventConditionContent {
            required_item_name: Some("Old Key".into()),
            required_condition_name: Some("Exhausted".into()),
            ..Default::default()
        };
        assert!(!matches_conditions(Some(&condition), &state, &context));
        state.character.inventory.push(crate::model::Item {
            id: 77,
            name: "Old Key".into(),
            description: "A rusted key.".into(),
        });
        state.character.conditions.push(crate::model::Condition::new("Exhausted", 2, -1));
        assert!(matches_conditions(Some(&condition), &state, &context));
    }

    #[test]
    fn reputation_condition_defaults_missing_faction_to_ineligible() {
        let state = test_state();
        let context = EventContext::for_travel_arrival("Ashen Gate", false, false);
        let condition = EventConditionContent { min_reputation: Some(1), ..Default::default() };
        assert!(!matches_conditions(Some(&condition), &state, &context));
    }

    #[test]
    fn prior_event_condition_uses_structured_history() {
        let mut state = test_state();
        let context = EventContext::for_travel_arrival("Ashen Gate", false, false);
        let condition = EventConditionContent {
            prior_event_id: Some("event.previous".into()),
            ..Default::default()
        };
        assert!(!matches_conditions(Some(&condition), &state, &context));
        state.world.record_event_history(0, "event.previous", "Ashen Gate", "The omen appeared.");
        assert!(matches_conditions(Some(&condition), &state, &context));
    }

    #[test]
    fn applied_event_records_structured_history() {
        let mut state = test_state();
        let event = EventContent {
            id: "event.test".into(),
            trigger: "travel_arrival".into(),
            weight: 1,
            chance_percent: Some(100),
            cooldown_turns: None,
            conditions: None,
            effects: vec![EventEffectContent::History { text: "A sign appears.".into() }],
        };
        let context = EventContext::for_travel_arrival("Ashen Gate", false, false);
        apply_event(&mut state, &event, &context);
        let entry = state.world.history.last().expect("event history should exist");
        assert_eq!(entry.entry_type, crate::model::HistoryEntryType::Event);
        assert_eq!(entry.event_id.as_deref(), Some("event.test"));
        assert_eq!(entry.location_name.as_deref(), Some("Ashen Gate"));
        assert!(entry.outcome.as_deref().unwrap().contains("A sign appears."));
    }

    #[test]
    fn cooldown_blocks_event_until_turn_is_reached() {
        let mut state = test_state();
        state.world.event_cooldowns.push(crate::model::EventCooldown { event_id: "test.event".into(), ready_at_turn: 3 });
        state.character.turn = 2;
        assert!(!event_is_off_cooldown(&state, "test.event"));
        state.character.turn = 3;
        assert!(event_is_off_cooldown(&state, "test.event"));
    }
}
