use crate::content::definitions::CampaignContent;
use crate::model::{Location, Region, World};

impl CampaignContent {
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

        let region_id = world
            .regions
            .first()
            .map(|region| region.id)
            .unwrap_or_else(|| {
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
                .filter_map(|exit_id| {
                    self.world
                        .locations
                        .iter()
                        .find(|candidate| candidate.id == *exit_id)
                })
                .filter_map(|exit_location| {
                    world
                        .location_by_name(&exit_location.name)
                        .map(|world_exit| world_exit.id)
                })
                .collect::<Vec<_>>();
            if let Some(world_location) = world.location_by_name_mut(&location.name) {
                world_location.exits = exits;
            }
        }

        if let Some(region) = world
            .regions
            .iter_mut()
            .find(|region| region.id == region_id)
        {
            region.location_ids = world
                .locations
                .iter()
                .filter(|location| location.region_id == region_id)
                .map(|location| location.id)
                .collect();
        }
    }
}
