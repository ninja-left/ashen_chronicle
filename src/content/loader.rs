use super::definitions::*;
use crate::ui;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const CONTENT_FILE_NAME: &str = "data/base_content.json";
const MODS_DIR_NAME: &str = "data/mods";

#[derive(Debug, Clone)]
struct DiscoveredMod {
    manifest: ModManifest,
    manifest_path: PathBuf,
}

impl ContentLoadReport {
    fn with_content(content: CampaignContent) -> Self {
        Self {
            content,
            loaded_mods: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

pub fn load_campaign_content() -> CampaignContent {
    load_campaign_content_report().content
}

pub fn load_campaign_content_report() -> ContentLoadReport {
    let base_content = match load_base_content() {
        Ok(content) => content,
        Err(err) => {
            ui::diagnostic(&format!("Could not load campaign content from disk: {err}"));
            default_campaign_content()
        }
    };
    let mut report = ContentLoadReport::with_content(base_content);
    let base_location_names: HashSet<&str> = report
        .content
        .world
        .locations
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();
    let base_faction_names: HashSet<&str> = report
        .content
        .factions
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();
    let base_events = std::mem::take(&mut report.content.events);
    let base_event_ids: HashSet<String> =
        base_events.iter().map(|event| event.id.clone()).collect();
    report.content.events = filter_valid_events(
        base_events,
        &base_location_names,
        &base_faction_names,
        &base_event_ids,
        &HashSet::<String>::new(),
        "base content",
        &mut report.warnings,
    );

    let mut discovered_mods = discover_mods();
    discovered_mods.sort_by(|left, right| {
        left.manifest
            .priority
            .cmp(&right.manifest.priority)
            .then_with(|| left.manifest.id.cmp(&right.manifest.id))
    });

    let mut seen_mod_ids = HashSet::new();
    for discovered in discovered_mods {
        if !discovered.manifest.enabled {
            continue;
        }
        if !seen_mod_ids.insert(discovered.manifest.id.clone()) {
            report.warnings.push(format!(
                "skipping duplicate mod id {}",
                discovered.manifest.id
            ));
            continue;
        }

        let manifest = discovered.manifest.clone();
        let manifest_id = manifest.id.clone();
        let manifest_name = manifest.name.clone();
        match load_mod_content(&discovered.manifest_path, &manifest) {
            Ok(mod_content) => {
                merge_campaign_content(&mut report.content, mod_content, &mut report.warnings);
                report.loaded_mods.push(manifest);
            }
            Err(err) => {
                report.warnings.push(format!(
                    "could not load mod {} ({}) from {}: {}",
                    manifest_id,
                    manifest_name,
                    discovered.manifest_path.display(),
                    err
                ));
            }
        }
    }

    report.warnings.extend(report.content.validate());
    report
}

fn load_base_content() -> io::Result<CampaignContent> {
    let path = campaign_content_path()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "base content file not found"))?;
    let data = fs::read_to_string(path)?;
    let parsed: CampaignContent = serde_json::from_str(&data)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?;
    Ok(parsed)
}

fn load_mod_content(manifest_path: &Path, manifest: &ModManifest) -> io::Result<CampaignContent> {
    let content_path = manifest_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(&manifest.content_file);
    let data = fs::read_to_string(&content_path)?;
    let parsed: CampaignContent = serde_json::from_str(&data)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?;
    Ok(parsed)
}

fn discover_mods() -> Vec<DiscoveredMod> {
    let mut found = Vec::new();
    let Some(mods_root) = mods_directory_path() else {
        return found;
    };

    let Ok(entries) = fs::read_dir(mods_root) else {
        return found;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let manifest_path = path.join("manifest.json");
        if !manifest_path.exists() {
            continue;
        }
        match fs::read_to_string(&manifest_path) {
            Ok(data) => match serde_json::from_str::<ModManifest>(&data) {
                Ok(manifest) => found.push(DiscoveredMod {
                    manifest,
                    manifest_path,
                }),
                Err(err) => ui::diagnostic(&format!(
                    "Could not parse mod manifest {}: {}",
                    manifest_path.display(),
                    err
                )),
            },
            Err(err) => ui::diagnostic(&format!(
                "Could not read mod manifest {}: {}",
                manifest_path.display(),
                err
            )),
        }
    }

    found
}

fn campaign_content_path() -> Option<PathBuf> {
    first_existing_path(CONTENT_FILE_NAME)
}

fn mods_directory_path() -> Option<PathBuf> {
    first_existing_path(MODS_DIR_NAME)
}

fn first_existing_path(relative: &str) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    candidates.push(PathBuf::from(relative));
    if let Ok(current_dir) = env::current_dir() {
        candidates.push(current_dir.join(relative));
    }
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join(relative));
            if let Some(parent) = dir.parent() {
                candidates.push(parent.join(relative));
            }
        }
    }
    candidates.into_iter().find(|path| Path::new(path).exists())
}

fn merge_campaign_content(
    base: &mut CampaignContent,
    incoming: CampaignContent,
    warnings: &mut Vec<String>,
) {
    base.world.region = incoming.world.region;
    merge_vec_by_key(
        &mut base.world.locations,
        incoming.world.locations,
        |entry| entry.id.clone(),
    );
    merge_vec_by_key(&mut base.factions, incoming.factions, |entry| {
        entry.id.clone()
    });
    merge_vec_by_key(&mut base.npcs, incoming.npcs, |entry| entry.id.clone());
    merge_vec_by_key(&mut base.quests, incoming.quests, |entry| entry.id.clone());
    merge_vec_by_key(&mut base.encounters, incoming.encounters, |entry| {
        entry.location_name.clone()
    });
    merge_vec_by_key(&mut base.atmospheres, incoming.atmospheres, |entry| {
        entry.location_name.clone()
    });
    merge_vec_by_key(&mut base.item_visuals, incoming.item_visuals, |entry| {
        entry.item_name.clone()
    });

    let location_names: HashSet<&str> = base
        .world
        .locations
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();
    let faction_names: HashSet<&str> = base
        .factions
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();
    let existing_ids: HashSet<String> = base.events.iter().map(|event| event.id.clone()).collect();
    let incoming_event_ids: HashSet<String> = incoming
        .events
        .iter()
        .map(|event| event.id.clone())
        .collect();
    let known_event_ids: HashSet<String> = existing_ids
        .iter()
        .cloned()
        .chain(incoming_event_ids.iter().cloned())
        .collect();
    let accepted_events = filter_valid_events(
        incoming.events,
        &location_names,
        &faction_names,
        &known_event_ids,
        &existing_ids,
        "mod content",
        warnings,
    );
    base.events.extend(accepted_events);
}

fn filter_valid_events(
    events: Vec<EventContent>,
    location_names: &HashSet<&str>,
    faction_names: &HashSet<&str>,
    known_ids: &HashSet<String>,
    existing_ids: &HashSet<String>,
    source: &str,
    warnings: &mut Vec<String>,
) -> Vec<EventContent> {
    let mut accepted = Vec::with_capacity(events.len());
    let mut seen_ids = existing_ids.clone();
    let mut pending = Vec::new();

    for event in events {
        let mut issues = validate_event_content(&event, location_names, faction_names, known_ids);
        if !seen_ids.insert(event.id.clone()) {
            issues.push(format!("duplicate event id {}", event.id));
        }
        if issues.is_empty() {
            pending.push(event);
        } else {
            warnings.push(format!(
                "rejecting event '{}' from {}: {}",
                event.id,
                source,
                issues.join("; ")
            ));
        }
    }

    loop {
        let accepted_ids: HashSet<String> = existing_ids
            .iter()
            .cloned()
            .chain(accepted.iter().map(|event: &EventContent| event.id.clone()))
            .chain(pending.iter().map(|event| event.id.clone()))
            .collect();
        let mut removed_any = false;
        let mut next_pending = Vec::with_capacity(pending.len());

        for event in pending {
            let missing_reference = event
                .conditions
                .as_ref()
                .and_then(|conditions| conditions.prior_event_id.as_deref())
                .filter(|prior_id| !accepted_ids.contains(*prior_id));

            if let Some(prior_id) = missing_reference {
                warnings.push(format!(
                    "rejecting event '{}' from {}: prior event '{}' was rejected or unavailable",
                    event.id, source, prior_id
                ));
                removed_any = true;
            } else {
                next_pending.push(event);
            }
        }

        pending = next_pending;
        if !removed_any {
            break;
        }
    }

    accepted.extend(pending);
    accepted
}

fn validate_event_content(
    event: &EventContent,
    location_names: &HashSet<&str>,
    faction_names: &HashSet<&str>,
    known_ids: &HashSet<String>,
) -> Vec<String> {
    let mut issues = Vec::new();
    if event.id.trim().is_empty() {
        issues.push("empty id".to_string());
    }
    if event.trigger.trim().is_empty() {
        issues.push("empty trigger".to_string());
    }
    if event.weight == 0 {
        issues.push("zero weight".to_string());
    }
    if let Some(chance) = event.chance_percent {
        if chance > 100 {
            issues.push(format!("invalid chance {}", chance));
        }
    }
    if event.effects.is_empty() {
        issues.push("no effects".to_string());
    }
    if let Some(conditions) = &event.conditions {
        for location in &conditions.locations {
            if !location_names.contains(location.as_str()) {
                issues.push(format!("unknown location {}", location));
            }
        }
        if let (Some(min_day), Some(max_day)) = (conditions.min_day, conditions.max_day) {
            if min_day > max_day {
                issues.push("min_day greater than max_day".to_string());
            }
        }
        if let Some(prior_event_id) = conditions.prior_event_id.as_deref() {
            if !known_ids.contains(prior_event_id) {
                issues.push(format!("unknown prior event id {}", prior_event_id));
            }
        }
        if let Some(faction_name) = conditions.faction_name.as_deref() {
            if !faction_names.contains(faction_name) {
                issues.push(format!("unknown faction {}", faction_name));
            }
        }
        if (conditions.min_reputation.is_some() || conditions.max_reputation.is_some())
            && conditions.faction_name.as_deref().is_none()
        {
            issues.push("reputation condition requires faction_name".to_string());
        }
        if let (Some(min_reputation), Some(max_reputation)) =
            (conditions.min_reputation, conditions.max_reputation)
        {
            if min_reputation > max_reputation {
                issues.push("min_reputation greater than max_reputation".to_string());
            }
        }
        if conditions
            .required_item_name
            .as_deref()
            .map(|name| name.trim().is_empty())
            .unwrap_or(false)
        {
            issues.push("required_item_name cannot be empty".to_string());
        }
        if conditions
            .required_condition_name
            .as_deref()
            .map(|name| name.trim().is_empty())
            .unwrap_or(false)
        {
            issues.push("required_condition_name cannot be empty".to_string());
        }
    }
    issues
}

fn merge_vec_by_key<T, F>(base: &mut Vec<T>, incoming: Vec<T>, key_fn: F)
where
    T: Clone,
    F: Fn(&T) -> String,
{
    for item in incoming {
        let key = key_fn(&item);
        if let Some(position) = base.iter().position(|existing| key_fn(existing) == key) {
            base[position] = item;
        } else {
            base.push(item);
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_mod_content_file() -> String {
    "content.json".to_string()
}

fn default_campaign_content() -> CampaignContent {
    serde_json::from_str(include_str!("../../data/base_content.json"))
        .expect("embedded base content JSON must remain valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_event(id: &str) -> EventContent {
        EventContent {
            id: id.to_string(),
            trigger: "travel_arrival".to_string(),
            weight: 1,
            chance_percent: Some(100),
            cooldown_turns: None,
            conditions: None,
            effects: vec![EventEffectContent::Pause],
        }
    }

    #[test]
    fn invalid_events_are_rejected_and_reported() {
        let mut warnings = Vec::new();
        let locations = HashSet::from(["Ashen Gate"]);
        let mut invalid = valid_event("bad.event");
        invalid.weight = 0;
        invalid.conditions = Some(EventConditionContent {
            locations: vec!["Unknown Place".to_string()],
            ..Default::default()
        });

        let known_ids = HashSet::from(["good.event".to_string(), "bad.event".to_string()]);
        let factions = HashSet::from(["Cinder Wardens"]);
        let accepted = filter_valid_events(
            vec![valid_event("good.event"), invalid],
            &locations,
            &factions,
            &known_ids,
            &HashSet::<String>::new(),
            "test content",
            &mut warnings,
        );

        assert_eq!(accepted.len(), 1);
        assert_eq!(accepted[0].id, "good.event");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("bad.event"));
        assert!(warnings[0].contains("zero weight"));
        assert!(warnings[0].contains("unknown location"));
    }

    #[test]
    fn invalid_prior_event_reference_rejects_dependent_content() {
        let mut warnings = Vec::new();
        let locations = HashSet::from(["Ashen Gate"]);
        let mut invalid = valid_event("bad.event");
        invalid.weight = 0;
        let mut dependent = valid_event("followup.event");
        dependent.conditions = Some(EventConditionContent {
            prior_event_id: Some("bad.event".to_string()),
            ..Default::default()
        });
        let known_ids = HashSet::from(["bad.event".to_string(), "followup.event".to_string()]);
        let factions = HashSet::from(["Cinder Wardens"]);
        let accepted = filter_valid_events(
            vec![invalid, dependent],
            &locations,
            &factions,
            &known_ids,
            &HashSet::<String>::new(),
            "test content",
            &mut warnings,
        );
        assert!(accepted.is_empty());
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("bad.event") && warning.contains("zero weight")));
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("followup.event")
                && warning.contains("was rejected or unavailable")));
    }

    #[test]
    fn prior_event_reference_can_target_existing_content() {
        let mut warnings = Vec::new();
        let locations = HashSet::from(["Ashen Gate"]);
        let factions = HashSet::from(["Cinder Wardens"]);
        let known_ids = HashSet::from(["base.event".to_string(), "followup.event".to_string()]);
        let existing = HashSet::from(["base.event".to_string()]);
        let mut followup = valid_event("followup.event");
        followup.conditions = Some(EventConditionContent {
            prior_event_id: Some("base.event".into()),
            ..Default::default()
        });
        let accepted = filter_valid_events(
            vec![followup],
            &locations,
            &factions,
            &known_ids,
            &existing,
            "mod content",
            &mut warnings,
        );
        assert_eq!(accepted.len(), 1);
        assert!(warnings.is_empty());
    }

    #[test]
    fn invalid_reputation_condition_is_rejected() {
        let mut warnings = Vec::new();
        let locations = HashSet::from(["Ashen Gate"]);
        let factions = HashSet::from(["Cinder Wardens"]);
        let known_ids = HashSet::from(["rep.event".to_string()]);
        let mut invalid = valid_event("rep.event");
        invalid.conditions = Some(EventConditionContent {
            min_reputation: Some(10),
            max_reputation: Some(5),
            ..Default::default()
        });
        let accepted = filter_valid_events(
            vec![invalid],
            &locations,
            &factions,
            &known_ids,
            &HashSet::<String>::new(),
            "test content",
            &mut warnings,
        );
        assert!(accepted.is_empty());
        assert!(warnings[0].contains("reputation condition requires faction_name"));
    }

    #[test]
    fn unknown_reputation_faction_is_rejected() {
        let mut warnings = Vec::new();
        let locations = HashSet::from(["Ashen Gate"]);
        let factions = HashSet::from(["Cinder Wardens"]);
        let known_ids = HashSet::from(["rep.event".to_string()]);
        let mut invalid = valid_event("rep.event");
        invalid.conditions = Some(EventConditionContent {
            faction_name: Some("Unknown Faction".into()),
            min_reputation: Some(1),
            ..Default::default()
        });
        let accepted = filter_valid_events(
            vec![invalid],
            &locations,
            &factions,
            &known_ids,
            &HashSet::<String>::new(),
            "test content",
            &mut warnings,
        );
        assert!(accepted.is_empty());
        assert!(warnings[0].contains("unknown faction"));
    }

    #[test]
    fn duplicate_event_ids_are_rejected_without_overwriting_existing_content() {
        let mut warnings = Vec::new();
        let locations = HashSet::from(["Ashen Gate"]);
        let existing = HashSet::from(["travel.event".to_string()]);

        let known_ids = HashSet::from(["travel.event".to_string(), "new.event".to_string()]);
        let factions = HashSet::from(["Cinder Wardens"]);
        let accepted = filter_valid_events(
            vec![valid_event("travel.event"), valid_event("new.event")],
            &locations,
            &factions,
            &known_ids,
            &existing,
            "mod content",
            &mut warnings,
        );

        assert_eq!(accepted.len(), 1);
        assert_eq!(accepted[0].id, "new.event");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("duplicate event id travel.event"));
    }
}
