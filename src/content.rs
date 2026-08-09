use crate::model::{Location, Region, World};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const CONTENT_FILE_NAME: &str = "data/base_content.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignContent {
    pub version: u32,
    pub world: WorldContent,
    #[serde(default)]
    pub factions: Vec<FactionContent>,
    #[serde(default)]
    pub npcs: Vec<NpcContent>,
    #[serde(default)]
    pub quests: Vec<QuestContent>,
    #[serde(default)]
    pub encounters: Vec<EncounterContent>,
    #[serde(default)]
    pub atmospheres: Vec<LocationAtmosphere>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldContent {
    pub region: RegionContent,
    pub locations: Vec<LocationContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionContent {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationContent {
    pub id: String,
    pub name: String,
    pub description: String,
    pub dangerous: bool,
    #[serde(default)]
    pub exits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactionContent {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcContent {
    pub id: String,
    pub name: String,
    pub title: String,
    pub location_name: String,
    #[serde(default)]
    pub faction_name: Option<String>,
    #[serde(default)]
    pub memory: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestContent {
    pub id: String,
    pub title: String,
    pub description: String,
    pub location_name: String,
    pub faction_name: String,
    pub giver_npc_name: String,
    pub required_item_name: String,
    pub reward_item_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncounterContent {
    pub location_name: String,
    pub enemy_name: String,
    pub enemy_hp: i32,
    pub enemy_power: i32,
    pub trophy_item_name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationAtmosphere {
    pub location_name: String,
    pub text: String,
}

impl CampaignContent {
    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();
        if self.version != 1 {
            issues.push(format!("content version {} is not recognized", self.version));
        }

        validate_unique_ids("location", self.world.locations.iter().map(|entry| entry.id.as_str()), &mut issues);
        validate_unique_ids("faction", self.factions.iter().map(|entry| entry.id.as_str()), &mut issues);
        validate_unique_ids("npc", self.npcs.iter().map(|entry| entry.id.as_str()), &mut issues);
        validate_unique_ids("quest", self.quests.iter().map(|entry| entry.id.as_str()), &mut issues);

        let location_names: HashSet<&str> = self.world.locations.iter().map(|entry| entry.name.as_str()).collect();
        let faction_names: HashSet<&str> = self.factions.iter().map(|entry| entry.name.as_str()).collect();
        let npc_names: HashSet<&str> = self.npcs.iter().map(|entry| entry.name.as_str()).collect();

        for location in &self.world.locations {
            for exit in &location.exits {
                if !self.world.locations.iter().any(|other| other.id == *exit) {
                    issues.push(format!("location {} exits to unknown location id {}", location.id, exit));
                }
            }
        }

        for npc in &self.npcs {
            if !location_names.contains(npc.location_name.as_str()) {
                issues.push(format!("npc {} uses unknown location {}", npc.id, npc.location_name));
            }
            if let Some(faction_name) = npc.faction_name.as_deref() {
                if !faction_names.contains(faction_name) {
                    issues.push(format!("npc {} uses unknown faction {}", npc.id, faction_name));
                }
            }
        }

        for quest in &self.quests {
            if !location_names.contains(quest.location_name.as_str()) {
                issues.push(format!("quest {} uses unknown location {}", quest.id, quest.location_name));
            }
            if !faction_names.contains(quest.faction_name.as_str()) {
                issues.push(format!("quest {} uses unknown faction {}", quest.id, quest.faction_name));
            }
            if !npc_names.contains(quest.giver_npc_name.as_str()) {
                issues.push(format!("quest {} uses unknown giver {}", quest.id, quest.giver_npc_name));
            }
        }

        for encounter in &self.encounters {
            if !location_names.contains(encounter.location_name.as_str()) {
                issues.push(format!("encounter {} uses unknown location {}", encounter.enemy_name, encounter.location_name));
            }
        }

        for atmosphere in &self.atmospheres {
            if !location_names.contains(atmosphere.location_name.as_str()) {
                issues.push(format!("atmosphere uses unknown location {}", atmosphere.location_name));
            }
        }

        issues
    }

    pub fn seed_world(&self, world: &mut World) {
        if world.regions.is_empty() {
            let region_id = world.allocate_id();
            world.regions.push(Region {
                id: region_id,
                name: self.world.region.name.clone(),
                description: self.world.region.description.clone(),
                location_ids: Vec::new(),
            });
        }

        let region_id = world.regions.first().map(|region| region.id).unwrap_or_else(|| {
            let id = world.allocate_id();
            world.regions.push(Region {
                id,
                name: self.world.region.name.clone(),
                description: self.world.region.description.clone(),
                location_ids: Vec::new(),
            });
            id
        });

        for location in &self.world.locations {
            if world.location_by_name(&location.name).is_none() {
                let id = world.allocate_id();
                world.locations.push(Location {
                    id,
                    name: location.name.clone(),
                    description: location.description.clone(),
                    region_id,
                    dangerous: location.dangerous,
                    corpse_ids: Vec::new(),
                    exits: Vec::new(),
                });
            }
        }

        for location in &self.world.locations {
            let exits = location
                .exits
                .iter()
                .filter_map(|exit_id| self.world.locations.iter().find(|candidate| candidate.id == *exit_id))
                .filter_map(|exit_location| world.location_by_name(&exit_location.name).map(|world_exit| world_exit.id))
                .collect::<Vec<_>>();
            if let Some(world_location) = world.location_by_name_mut(&location.name) {
                world_location.exits = exits;
            }
        }

        if let Some(region) = world.regions.iter_mut().find(|region| region.id == region_id) {
            region.location_ids = world
                .locations
                .iter()
                .filter(|location| location.region_id == region_id)
                .map(|location| location.id)
                .collect();
        }
    }

    pub fn atmosphere_for(&self, location_name: &str) -> Option<&str> {
        self.atmospheres
            .iter()
            .find(|entry| entry.location_name == location_name)
            .map(|entry| entry.text.as_str())
    }

    pub fn encounter_for(&self, location_name: &str) -> Option<&EncounterContent> {
        self.encounters.iter().find(|entry| entry.location_name == location_name)
    }
}

fn validate_unique_ids<'a, I>(kind: &str, ids: I, issues: &mut Vec<String>)
where
    I: Iterator<Item = &'a str>,
{
    let mut seen = HashSet::new();
    for id in ids {
        if !seen.insert(id) {
            issues.push(format!("duplicate {} id {}", kind, id));
        }
    }
}

pub fn load_campaign_content() -> CampaignContent {
    if let Ok(content) = load_from_disk() {
        let issues = content.validate();
        if issues.is_empty() {
            return content;
        }
        eprintln!("Campaign content warnings:");
        for issue in issues {
            eprintln!("- {issue}");
        }
        return content;
    }
    default_campaign_content()
}

fn load_from_disk() -> io::Result<CampaignContent> {
    let path = campaign_content_path().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "base content file not found"))?;
    let data = fs::read_to_string(path)?;
    let parsed: CampaignContent = serde_json::from_str(&data)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?;
    Ok(parsed)
}

fn campaign_content_path() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    candidates.push(PathBuf::from(CONTENT_FILE_NAME));
    if let Ok(current_dir) = env::current_dir() {
        candidates.push(current_dir.join(CONTENT_FILE_NAME));
    }
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join(CONTENT_FILE_NAME));
            if let Some(parent) = dir.parent() {
                candidates.push(parent.join(CONTENT_FILE_NAME));
            }
        }
    }
    candidates.into_iter().find(|path| Path::new(path).exists())
}

fn default_campaign_content() -> CampaignContent {
    CampaignContent {
        version: 1,
        world: WorldContent {
            region: RegionContent {
                id: "region.ashen_crown".to_string(),
                name: "The Ashen Crown".to_string(),
                description: "A bleak frontier where old stone roads still cut through soot and cinder.".to_string(),
            },
            locations: vec![
                LocationContent {
                    id: "location.ashen_gate".to_string(),
                    name: "Ashen Gate".to_string(),
                    description: "A cracked iron gate hanging between broken towers, staring over a dead road.".to_string(),
                    dangerous: false,
                    exits: vec!["location.hollow_market".to_string(), "location.charred_watchtower".to_string()],
                },
                LocationContent {
                    id: "location.hollow_market".to_string(),
                    name: "Hollow Market".to_string(),
                    description: "Stalls without merchants, lanterns without flame, and the echo of old bargaining.".to_string(),
                    dangerous: false,
                    exits: vec!["location.ashen_gate".to_string(), "location.old_shrine".to_string(), "location.sootbound_crossing".to_string()],
                },
                LocationContent {
                    id: "location.old_shrine".to_string(),
                    name: "Old Shrine".to_string(),
                    description: "A roofless shrine with a black altar and fresh soot on the floor.".to_string(),
                    dangerous: true,
                    exits: vec!["location.hollow_market".to_string(), "location.mourning_fields".to_string()],
                },
                LocationContent {
                    id: "location.charred_watchtower".to_string(),
                    name: "Charred Watchtower".to_string(),
                    description: "A leaning watchtower with a bell that rings when the wind changes.".to_string(),
                    dangerous: false,
                    exits: vec!["location.ashen_gate".to_string(), "location.mourning_fields".to_string()],
                },
                LocationContent {
                    id: "location.mourning_fields".to_string(),
                    name: "Mourning Fields".to_string(),
                    description: "A field of ash where pale grass grows around old burial stones.".to_string(),
                    dangerous: false,
                    exits: vec!["location.old_shrine".to_string(), "location.charred_watchtower".to_string(), "location.blackroot_hollow".to_string()],
                },
                LocationContent {
                    id: "location.blackroot_hollow".to_string(),
                    name: "Blackroot Hollow".to_string(),
                    description: "A low ravine choked with black roots and the smell of wet iron.".to_string(),
                    dangerous: true,
                    exits: vec!["location.mourning_fields".to_string(), "location.drowned_chapel".to_string()],
                },
                LocationContent {
                    id: "location.drowned_chapel".to_string(),
                    name: "Drowned Chapel".to_string(),
                    description: "A half-sunken chapel whose bell chamber disappears beneath dark water.".to_string(),
                    dangerous: true,
                    exits: vec!["location.blackroot_hollow".to_string(), "location.sootbound_crossing".to_string()],
                },
                LocationContent {
                    id: "location.sootbound_crossing".to_string(),
                    name: "Sootbound Crossing".to_string(),
                    description: "A ruined road crossing where caravan tracks vanish into the cinder.".to_string(),
                    dangerous: false,
                    exits: vec!["location.hollow_market".to_string(), "location.drowned_chapel".to_string()],
                },
            ],
        },
        factions: vec![
            FactionContent { id: "faction.cinder_wardens".to_string(), name: "Cinder Wardens".to_string() },
            FactionContent { id: "faction.hollow_market_kin".to_string(), name: "Hollow Market Kin".to_string() },
            FactionContent { id: "faction.drowned_bell_covenant".to_string(), name: "Drowned Bell Covenant".to_string() },
        ],
        npcs: vec![
            NpcContent {
                id: "npc.mira".to_string(),
                name: "Mira".to_string(),
                title: "Scout".to_string(),
                location_name: "Hollow Market".to_string(),
                faction_name: Some("Cinder Wardens".to_string()),
                memory: vec!["Keeps watch on the shrine road.".to_string()],
            },
            NpcContent {
                id: "npc.bram".to_string(),
                name: "Bram".to_string(),
                title: "Gatekeeper".to_string(),
                location_name: "Ashen Gate".to_string(),
                faction_name: Some("Hollow Market Kin".to_string()),
                memory: vec!["Counts every traveler who passes the gate.".to_string()],
            },
            NpcContent {
                id: "npc.ilyra".to_string(),
                name: "Ilyra".to_string(),
                title: "Bell Keeper".to_string(),
                location_name: "Drowned Chapel".to_string(),
                faction_name: Some("Drowned Bell Covenant".to_string()),
                memory: vec!["Listens for bells beneath the water.".to_string()],
            },
            NpcContent {
                id: "npc.tovin".to_string(),
                name: "Tovin".to_string(),
                title: "Grave Tender".to_string(),
                location_name: "Mourning Fields".to_string(),
                faction_name: Some("Cinder Wardens".to_string()),
                memory: vec!["Marks graves that the ash has not swallowed.".to_string()],
            },
            NpcContent {
                id: "npc.kes".to_string(),
                name: "Kes".to_string(),
                title: "Root Gatherer".to_string(),
                location_name: "Blackroot Hollow".to_string(),
                faction_name: Some("Hollow Market Kin".to_string()),
                memory: vec!["Trades medicines made from blackroot.".to_string()],
            },
        ],
        quests: vec![
            QuestContent {
                id: "quest.quiet_old_shrine".to_string(),
                title: "Quiet the Old Shrine".to_string(),
                description: "The wardens want the shrine cleared of whatever woke there.".to_string(),
                location_name: "Old Shrine".to_string(),
                faction_name: "Cinder Wardens".to_string(),
                giver_npc_name: "Mira".to_string(),
                required_item_name: "Trophy from Old Shrine".to_string(),
                reward_item_name: "Wardens' Seal".to_string(),
            },
            QuestContent {
                id: "quest.roots_for_market".to_string(),
                title: "Roots for the Market".to_string(),
                description: "Kes wants a fresh blackroot cutting from the hollow before the roots rot.".to_string(),
                location_name: "Blackroot Hollow".to_string(),
                faction_name: "Hollow Market Kin".to_string(),
                giver_npc_name: "Kes".to_string(),
                required_item_name: "Rootbound Fang".to_string(),
                reward_item_name: "Rootworker's Token".to_string(),
            },
            QuestContent {
                id: "quest.drowned_bell".to_string(),
                title: "The Drowned Bell".to_string(),
                description: "Ilyra asks you to recover the bell clapper from the drowned chapel.".to_string(),
                location_name: "Drowned Chapel".to_string(),
                faction_name: "Drowned Bell Covenant".to_string(),
                giver_npc_name: "Ilyra".to_string(),
                required_item_name: "Drowned Rosary".to_string(),
                reward_item_name: "Bell Covenant Charm".to_string(),
            },
        ],
        encounters: vec![
            EncounterContent {
                location_name: "Old Shrine".to_string(),
                enemy_name: "Ashen Wretch".to_string(),
                enemy_hp: 6,
                enemy_power: 2,
                trophy_item_name: "Trophy from Old Shrine".to_string(),
                description: "A bent thing of ash and hunger tears at the altar steps.".to_string(),
            },
            EncounterContent {
                location_name: "Blackroot Hollow".to_string(),
                enemy_name: "Rootbound Stalker".to_string(),
                enemy_hp: 8,
                enemy_power: 2,
                trophy_item_name: "Rootbound Fang".to_string(),
                description: "Something with bark-hard limbs stalks between the roots.".to_string(),
            },
            EncounterContent {
                location_name: "Drowned Chapel".to_string(),
                enemy_name: "Drowned Penitent".to_string(),
                enemy_hp: 10,
                enemy_power: 3,
                trophy_item_name: "Drowned Rosary".to_string(),
                description: "A waterlogged shape rises from the nave, dragging a chain of prayer beads.".to_string(),
            },
        ],
        atmospheres: vec![
            LocationAtmosphere { location_name: "Ashen Gate".to_string(), text: "Wind slips through the broken towers, carrying the smell of cold iron.".to_string() },
            LocationAtmosphere { location_name: "Hollow Market".to_string(), text: "A shutter moves by itself. Somewhere behind the empty stalls, coins clink once.".to_string() },
            LocationAtmosphere { location_name: "Old Shrine".to_string(), text: "Ash gathers in the altar's cracks. Whatever stirred here has not forgotten the road.".to_string() },
            LocationAtmosphere { location_name: "Charred Watchtower".to_string(), text: "The watchtower bell gives a single dull knock, though no hand touches it.".to_string() },
            LocationAtmosphere { location_name: "Mourning Fields".to_string(), text: "Pale grass bends around old stones, exposing scraps of names beneath the ash.".to_string() },
            LocationAtmosphere { location_name: "Blackroot Hollow".to_string(), text: "Black roots shift under the soil with a sound like distant breathing.".to_string() },
            LocationAtmosphere { location_name: "Drowned Chapel".to_string(), text: "Water laps against the chapel steps. Far below, something answers with a bell note.".to_string() },
            LocationAtmosphere { location_name: "Sootbound Crossing".to_string(), text: "Old wheel tracks divide at the crossing, then vanish where the ash has been disturbed.".to_string() },
        ],
    }
}
