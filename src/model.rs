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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub world: World,
    pub character: Character,
    #[serde(default)]
    pub threat: ThreatState,
    #[serde(default)]
    pub corpses: Vec<Corpse>,
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
        let region = Region {
            id: region_id,
            name: "The Ashen Crown".to_string(),
            description: "A bleak frontier where old stone roads still cut through soot and cinder.".to_string(),
            location_ids: Vec::new(),
        };

        let gate_id = self.allocate_id();
        let market_id = self.allocate_id();
        let shrine_id = self.allocate_id();

        let gate = Location {
            id: gate_id,
            name: "Ashen Gate".to_string(),
            description: "A cracked iron gate hanging between broken towers, staring over a dead road.".to_string(),
            region_id,
            dangerous: false,
            corpse_ids: Vec::new(),
            exits: vec![market_id],
        };
        let market = Location {
            id: market_id,
            name: "Hollow Market".to_string(),
            description: "Stalls without merchants, lanterns without flame, and the echo of old bargaining.".to_string(),
            region_id,
            dangerous: false,
            corpse_ids: Vec::new(),
            exits: vec![gate_id, shrine_id],
        };
        let shrine = Location {
            id: shrine_id,
            name: "Old Shrine".to_string(),
            description: "A roofless shrine with a black altar and fresh soot on the floor.".to_string(),
            region_id,
            dangerous: true,
            corpse_ids: Vec::new(),
            exits: vec![market_id],
        };

        self.regions.push(region);
        self.locations.push(gate);
        self.locations.push(market);
        self.locations.push(shrine);
        if let Some(region) = self.regions.first_mut() {
            region.location_ids = vec![gate_id, market_id, shrine_id];
        }
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

pub fn create_new_state(world_name: &str, mode: WorldMode, character_name: String, title: String) -> GameState {
    let mut world = World::new(world_name, mode);
    let character = world.spawn_character(character_name, title);
    world.record_history(0, format!("{} entered the world.", character.display_name()));
    GameState {
        world,
        character,
        threat: ThreatState::default(),
        corpses: Vec::new(),
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
    }
}
