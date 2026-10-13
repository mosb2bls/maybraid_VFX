//! Complete character scene recipes and their entity-dependent fulfillment.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;
use character_inventory_user::spawn_bag;
use character_items::{CharacterSheet, Inventory, InventoryItem};
use damage::Health;
use firearm_user::{live_weapon_from_stats, spawn_held_kit, FirearmUserSettings};
use intelligence_lod::IntelligenceLod;
use mob_intelligence::{MobMemberBody, MobSlot, MobSystems};
use npc_intelligence::{NpcBody, NpcInstall};
use player::{
	apply_character_controller, apply_character_mobility, Npc, PlayerLook, PlayerYawOwner,
};
use routing_intelligence::{RoutingIntelligenceUser, RoutingSettings};
use spotting_intelligence::{InterestLayers, SpotBounds, SpotSubject};

use crate::{
	CharacterBrains, CharacterBuild, CharacterInventory, CharacterSpecies, FromMobNumber,
	GeneratedFirearm,
};

const ROUTING_BANDS: [f32; 3] = [160.0, 80.0, 32.0];

/// Authored constructor shape from the mobs sketch.
#[derive(Clone, Debug, PartialEq)]
pub struct MobCharacter<
	Build = CharacterBuild,
	Species = CharacterSpecies,
	InventoryProfile = CharacterInventory,
	Brains = CharacterBrains,
> {
	pub num: f32,
	pub build: Build,
	pub species: Species,
	pub inventory: InventoryProfile,
	pub brains: Brains,
}

impl<Build, Species, InventoryProfile, Brains>
	MobCharacter<Build, Species, InventoryProfile, Brains>
where
	Build: FromMobNumber,
	Species: FromMobNumber,
	InventoryProfile: FromMobNumber,
	Brains: FromMobNumber,
{
	pub fn from_num(num: f32) -> Self {
		Self {
			num,
			build: Build::from_num(num),
			species: Species::from_num(num),
			inventory: InventoryProfile::from_num(num),
			brains: Brains::from_num(num),
		}
	}
}

impl MobCharacter {
	pub fn scene_recipe(&self) -> CharacterSceneRecipe {
		let inventory_profile = if self.species.supports_inventory() {
			self.inventory
		} else {
			CharacterInventory::Empty
		};
		CharacterSceneRecipe {
			num: self.num,
			build: self.build,
			species: self.species,
			inventory_profile,
			inventory: inventory_profile.generate(self.num),
			brains: self.brains,
		}
	}
}

#[derive(Component, Clone, Debug, PartialEq)]
pub struct CharacterSceneRecipe {
	pub num: f32,
	pub build: CharacterBuild,
	pub species: CharacterSpecies,
	pub inventory_profile: CharacterInventory,
	pub inventory: Inventory,
	pub brains: CharacterBrains,
}

impl Default for CharacterSceneRecipe {
	fn default() -> Self {
		MobCharacter::from_num(0.0).scene_recipe()
	}
}

impl CharacterSceneRecipe {
	pub fn sheet(&self) -> CharacterSheet {
		self.build.apply(self.inventory.character_sheet())
	}

	pub fn armed(&self) -> bool {
		self.inventory.primary_weapon().is_some()
	}

	pub fn locomotion_capsule(&self) -> characters::LocomotionCapsule {
		self.species.model(self.build, &self.inventory).hull()
	}

	pub fn spawn(&self, commands: &mut Commands, transform: Transform) -> Entity {
		commands
			.spawn((
				Name::new("mob-character-scene"),
				self.clone(),
				transform,
				Visibility::default(),
			))
			.id()
	}
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CharacterStats(pub CharacterSheet);

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CharacterSceneSystems {
	Materialize,
}

pub(crate) fn materialize_character_scenes(
	mut commands: Commands,
	scenes: Query<
		(Entity, &CharacterSceneRecipe, &Transform, Has<MobSlot>),
		Added<CharacterSceneRecipe>,
	>,
) {
	for (body, recipe, transform, belongs_to_mob) in &scenes {
		let model = recipe.species.model(recipe.build, &recipe.inventory);
		let hull = model.hull();
		let npc_body = NpcBody {
			agent_radius: hull.radius,
			feet_below_origin: hull.half_height(),
			eye_height: (hull.length + 2.0 * hull.radius) * 0.8,
		};
		let sheet = recipe.sheet();
		let health = Health::from_max(f32::from(sheet.health));
		apply_character_controller(&mut commands, body, hull);
		apply_character_mobility(
			&mut commands,
			body,
			f32::from(sheet.running) / f32::from(CharacterSheet::BASE.running),
			f32::from(sheet.jump) / f32::from(CharacterSheet::BASE.jump),
		);
		commands.entity(body).insert((
			Name::new(format!("{:?} {:?}", recipe.species, recipe.brains)),
			Npc,
			PlayerLook::default(),
			PlayerYawOwner::Wish,
			recipe.build,
			recipe.species,
			recipe.brains,
			CharacterStats(sheet),
			MobMemberBody(npc_body),
			SpotSubject::new(
				InterestLayers::CHARACTER,
				SpotBounds::capsule(hull.radius, hull.half_height()),
			),
			health,
			recipe.species.flinch_profile(),
		));
		spawn_bag(&mut commands, body, recipe.inventory.clone());
		model.spawn_visual(&mut commands, body, Quat::from_rotation_y(FRAC_PI_2));

		if let Some(InventoryItem::Firearm { spec, stats }) = recipe.inventory.primary_weapon() {
			let live = live_weapon_from_stats(*stats, sheet.damage).with_weapon_identity(spec);
			spawn_held_kit(
				&mut commands,
				body,
				FirearmUserSettings { aim_yaw_limit: PI, ..default() },
				GeneratedFirearm::from_spec(*spec),
				live,
			);
		}

		if belongs_to_mob {
			commands.entity(body).insert(IntelligenceLod::missing());
		}
		if recipe.brains.uses_long_range_routing() && !belongs_to_mob {
			commands.entity(body).insert(RoutingIntelligenceUser::new(
				RoutingSettings::from_segments(ROUTING_BANDS),
			));
		}
		if !belongs_to_mob {
			recipe.brains.personality(recipe.armed()).install(
				&mut commands,
				body,
				NpcInstall {
					at: transform.translation,
					body: npc_body,
					health,
					armed: recipe.armed(),
					poi_interests: recipe.brains.interests(),
					keep_tether_in_combat: Some(recipe.brains.keep_tether_in_combat()),
					..default()
				},
			);
		}
	}
}

pub(crate) fn configure_character_scene_systems(app: &mut App) {
	app.configure_sets(Update, CharacterSceneSystems::Materialize.before(MobSystems::Bind))
		.add_systems(
			Update,
			materialize_character_scenes.in_set(CharacterSceneSystems::Materialize),
		);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn quadrupeds_drop_incompatible_inventory() {
		let character = MobCharacter {
			num: 3.0,
			build: CharacterBuild::Tank,
			species: CharacterSpecies::Epiphant,
			inventory: CharacterInventory::Mist,
			brains: CharacterBrains::Guard,
		};
		let recipe = character.scene_recipe();
		assert_eq!(recipe.inventory_profile, CharacterInventory::Empty);
		assert!(recipe.inventory.items.is_empty());
	}

	#[test]
	fn generated_character_retains_inventory_and_build_stats() {
		let character = MobCharacter {
			num: 9.0,
			build: CharacterBuild::Master,
			species: CharacterSpecies::Braidman,
			inventory: CharacterInventory::Grunt,
			brains: CharacterBrains::Brawler,
		};
		let recipe = character.scene_recipe();
		assert!(recipe.armed());
		assert!(recipe.sheet().health > CharacterSheet::BASE.health);
	}

	#[test]
	fn player_scale_bipeds_are_the_bipeds_in_the_player_capsule_band() {
		for species in CharacterSpecies::BIPEDS {
			let capsule = MobCharacter {
				num: 1.0,
				build: CharacterBuild::Base,
				species,
				inventory: CharacterInventory::Grunt,
				brains: CharacterBrains::Brawler,
			}
			.scene_recipe()
			.locomotion_capsule();
			let height = capsule.length + 2.0 * capsule.radius;
			let in_band = (1.4..=2.1).contains(&height) && capsule.radius <= 0.5;
			assert_eq!(
				CharacterSpecies::PLAYER_SCALE_BIPEDS.contains(&species),
				in_band,
				"{species:?}: {height:.2} m tall, {:.2} m radius",
				capsule.radius
			);
		}
	}
}
