use serde::{Deserialize, Serialize};

pub type EntityId = u64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorldMode {
    New,
    Inherited,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThreatState {
    pub active: bool,
    pub source_location_id: Option<EntityId>,
    pub label: String,
    pub description: String,
}

impl ThreatState {
    pub fn activate(
        &mut self,
        source_location_id: EntityId,
        label: impl Into<String>,
        description: impl Into<String>,
    ) {
        self.active = true;
        self.source_location_id = Some(source_location_id);
        self.label = label.into();
        self.description = description.into();
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct World {
    pub id: EntityId,
    pub name: String,
    pub mode: WorldMode,
    pub next_id: EntityId,
    pub regions: Vec<Region>,
    pub locations: Vec<Location>,
    pub history: Vec<HistoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    pub id: EntityId,
    pub name: String,
    pub description: String,
    pub location_ids: Vec<EntityId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub id: EntityId,
    pub name: String,
    pub description: String,
    pub region_id: EntityId,
    #[serde(default)]
    pub dangerous: bool,
    #[serde(default)]
    pub corpse_ids: Vec<EntityId>,
    pub exits: Vec<EntityId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: EntityId,
    pub turn: u32,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: EntityId,
    pub name: String,
    pub title: String,
    pub hp: i32,
    pub max_hp: i32,
    pub location_id: EntityId,
    pub inventory: Vec<Item>,
    pub alive: bool,
    pub turn: u32,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: EntityId,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Corpse {
    pub id: EntityId,
    pub former_name: String,
    pub former_title: String,
    pub location_id: EntityId,
    pub turn_of_death: u32,
    pub inventory: Vec<Item>,
    pub epitaph: String,
    #[serde(default)]
    pub scavenged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Faction {
    pub id: EntityId,
    pub name: String,
    #[serde(default)]
    pub reputation: i32,
    #[serde(default)]
    pub memory: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Npc {
    pub id: EntityId,
    pub name: String,
    pub title: String,
    pub location_id: EntityId,
    #[serde(default)]
    pub faction_id: Option<EntityId>,
    #[serde(default)]
    pub memory: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Quest {
    pub id: EntityId,
    pub title: String,
    pub description: String,
    pub target_location_id: EntityId,
    pub faction_id: EntityId,
    #[serde(default)]
    pub giver_npc_id: EntityId,
    #[serde(default)]
    pub required_item_name: String,
    #[serde(default)]
    pub completed_by: Option<String>,
    #[serde(default)]
    pub offered: bool,
    #[serde(default)]
    pub completed: bool,
    #[serde(default)]
    pub reward_claimed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub world: World,
    pub character: Character,
    #[serde(default)]
    pub threat: ThreatState,
    #[serde(default)]
    pub corpses: Vec<Corpse>,
    #[serde(default)]
    pub factions: Vec<Faction>,
    #[serde(default)]
    pub npcs: Vec<Npc>,
    #[serde(default)]
    pub quests: Vec<Quest>,
    #[serde(default)]
    pub last_announced_location_id: Option<EntityId>,
}

impl World {
    pub fn new(name: &str, mode: WorldMode) -> Self {
        let mut world = Self {
            id: 1,
            name: name.to_string(),
            mode,
            next_id: 2,
            regions: Vec::new(),
            locations: Vec::new(),
            history: Vec::new(),
        };
        world.seed_demo_world();
        world.record_history(0, "A new world stirs beneath ash and ruin.");
        world
    }

    pub fn allocate_id(&mut self) -> EntityId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn region_by_id(&self, id: EntityId) -> Option<&Region> {
        self.regions.iter().find(|region| region.id == id)
    }

    pub fn location_by_id(&self, id: EntityId) -> Option<&Location> {
        self.locations.iter().find(|location| location.id == id)
    }

    pub fn location_by_id_mut(&mut self, id: EntityId) -> Option<&mut Location> {
        self.locations.iter_mut().find(|location| location.id == id)
    }

    pub fn location_is_dangerous(&self, id: EntityId) -> bool {
        self.location_by_id(id).map(|location| location.dangerous).unwrap_or(false)
    }

    pub fn record_history(&mut self, turn: u32, text: impl Into<String>) {
        let entry = HistoryEntry {
            id: self.allocate_id(),
            turn,
            text: text.into(),
        };
        self.history.push(entry);
    }

    pub fn spawn_character(&mut self, name: String, title: String) -> Character {
        let location_id = self.locations.first().map(|loc| loc.id).unwrap_or(0);
        let character_id = self.allocate_id();
        Character::new(character_id, name, title, location_id)
    }

    fn seed_demo_world(&mut self) {
        let region_id = self.allocate_id();
        self.regions.push(Region {
            id: region_id,
            name: "The Ashen Crown".to_string(),
            description: "A bleak frontier where old stone roads still cut through soot and cinder.".to_string(),
            location_ids: Vec::new(),
        });

        let specs = [
            ("Ashen Gate", "A cracked iron gate hanging between broken towers, staring over a dead road.", false),
            ("Hollow Market", "Stalls without merchants, lanterns without flame, and the echo of old bargaining.", false),
            ("Old Shrine", "A roofless shrine with a black altar and fresh soot on the floor.", true),
            ("Charred Watchtower", "A leaning watchtower with a bell that rings when the wind changes.", false),
            ("Mourning Fields", "A field of ash where pale grass grows around old burial stones.", false),
            ("Blackroot Hollow", "A low ravine choked with black roots and the smell of wet iron.", true),
            ("Drowned Chapel", "A half-sunken chapel whose bell chamber disappears beneath dark water.", true),
            ("Sootbound Crossing", "A ruined road crossing where caravan tracks vanish into the cinder.", false),
        ];
        let ids: Vec<EntityId> = specs.iter().map(|(name, description, dangerous)| {
            let id = self.allocate_id();
            self.locations.push(Location { id, name: (*name).to_string(), description: (*description).to_string(), region_id, dangerous: *dangerous, corpse_ids: Vec::new(), exits: Vec::new() });
            id
        }).collect();
        let (gate, market, shrine, tower, fields, hollow, chapel, crossing) = (ids[0],ids[1],ids[2],ids[3],ids[4],ids[5],ids[6],ids[7]);
        let exits = [
            (gate, vec![market,tower]), (market,vec![gate,shrine,crossing]), (shrine,vec![market,fields]),
            (tower,vec![gate,fields]), (fields,vec![shrine,tower,hollow]), (hollow,vec![fields,chapel]),
            (chapel,vec![hollow,crossing]), (crossing,vec![market,chapel]),
        ];
        for (id, targets) in exits { if let Some(location) = self.location_by_id_mut(id) { location.exits = targets; } }
        self.regions[0].location_ids = ids;
    }
}

impl Character {
    pub fn new(id: EntityId, name: String, title: String, location_id: EntityId) -> Self {
        Self {
            id,
            name,
            title,
            hp: 10,
            max_hp: 10,
            location_id,
            inventory: Vec::new(),
            alive: true,
            turn: 0,
            notes: vec!["Born into ash, with no past worth keeping.".to_string()],
        }
    }

    pub fn display_name(&self) -> String {
        format!("{} the {}", self.name, self.title)
    }

    pub fn heal(&mut self, amount: i32) {
        self.hp = (self.hp + amount).min(self.max_hp);
    }
}

impl Faction {
    pub fn new(id: EntityId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            reputation: 0,
            memory: Vec::new(),
        }
    }
}

impl Npc {
    pub fn new(
        id: EntityId,
        name: impl Into<String>,
        title: impl Into<String>,
        location_id: EntityId,
        faction_id: Option<EntityId>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            title: title.into(),
            location_id,
            faction_id,
            memory: Vec::new(),
        }
    }

    pub fn display_name(&self) -> String {
        format!("{} the {}", self.name, self.title)
    }
}

impl Quest {
    pub fn new(
        id: EntityId,
        title: impl Into<String>,
        description: impl Into<String>,
        target_location_id: EntityId,
        faction_id: EntityId,
        giver_npc_id: EntityId,
        required_item_name: impl Into<String>,
    ) -> Self {
        Self {
            id,
            title: title.into(),
            description: description.into(),
            target_location_id,
            faction_id,
            giver_npc_id,
            required_item_name: required_item_name.into(),
            completed_by: None,
            offered: false,
            completed: false,
            reward_claimed: false,
        }
    }
}

pub fn create_new_state(world_name: &str, mode: WorldMode, character_name: String, title: String) -> GameState {
    let mut world = World::new(world_name, mode);
    let character = world.spawn_character(character_name, title);
    world.record_history(0, format!("{} entered the world.", character.display_name()));
    GameState {
        world,
        character,
        threat: ThreatState::default(),
        corpses: Vec::new(),
        factions: Vec::new(),
        npcs: Vec::new(),
        quests: Vec::new(),
        last_announced_location_id: None,
    }
}

pub fn create_inherited_state(state: &GameState, character_name: String, title: String) -> GameState {
    let mut world = state.world.clone();
    world.mode = WorldMode::Inherited;
    let character = world.spawn_character(character_name, title);
    let turn = world.history.last().map(|entry| entry.turn).unwrap_or(0);
    world.record_history(turn, format!("{} inherited the world.", character.display_name()));
    GameState {
        world,
        character,
        threat: ThreatState::default(),
        corpses: state.corpses.clone(),
        factions: state.factions.clone(),
        npcs: state.npcs.clone(),
        quests: state.quests.clone(),
        last_announced_location_id: None,
    }
}
