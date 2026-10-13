//! Common bedroom usage area: beds / nightstands + walled closet / ensuite rooms.
//!
//! **Semantically:** a residential cell with passage keep-outs, sleep furniture,
//! and private partitions (closet, ensuite) that open onto the bedroom floor.
//!
//! **Programmatically:**
//! 1. [`CommonBedroomParameterized::sample`] (or [`CommonBedroomParameterized::with_fill`])
//! 2. [`CommonBedroomPlan::from_parameterized`] packs clearances → beds →
//!    nightstands → [`crate::usage_areas::enclosed_room`] closets / ensuites
//! 3. [`CommonBedroom`] presents furniture + partition panels; residual
//!    [`FillableRegions::within`] carry closet / ensuite confines with doors

mod layout;
mod parameterized;

#[cfg(test)]
mod tests;

pub use parameterized::{CommonBedroomParameterized, CommonBedroomPlan, SCOPE};

use building_components::furniture::FurnitureNode;
use building_components::panels::PanelNode;
use building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};
use lod::scene::LodSceneLevel;
use procedural_common::NoiseParams;

use crate::fit::{Confines, FillRegion, FillableRegions, Fit, FitError, SpaceKind};
use crate::paneling::Rectangle;
use crate::placer::WalledRoomFill;
use crate::usage_areas::furniture_util::furniture_fill;
use crate::usage_areas::label_util::label_filling_aabb;
use crate::usage_areas::livable_quarters::ResidentialBathroom;

use layout::BedroomPartition;

/// Presentable common bedroom: furniture + closet / ensuite enclosure panels.
#[derive(Debug, Clone, PartialEq)]
pub struct CommonBedroom {
	pub room_type: LabelNode,
	pub beds: Vec<BedFill>,
	pub nightstands: Vec<NightstandFill>,
	pub small_bedroom_furniture: Vec<SmallBedroomFurnitureFill>,
	pub wardrobes: Vec<WardrobeFill>,
	pub dressers: Vec<DresserFill>,
	pub bedroom_furniture: Vec<BedroomFurnitureFill>,
	pub closet_walls: Vec<Rectangle>,
	pub ensuite_walls: Vec<Rectangle>,
	pub closets: Vec<ClosetFill>,
	pub walk_in_closets: Vec<WalkInClosetFill>,
	pub ensuites: Vec<EnsuiteFill>,
	/// Bathrooms composed into ensuite within regions (when fit succeeds).
	pub ensuite_bathrooms: Vec<ResidentialBathroom>,
}

/// Bed furniture + AABB label.
#[derive(Debug, Clone, PartialEq)]
pub struct BedFill {
	pub label: LabelNode,
	pub furniture: FurnitureNode,
}

/// Nightstand furniture + AABB label (bed-adjacent only).
#[derive(Debug, Clone, PartialEq)]
pub struct NightstandFill {
	pub label: LabelNode,
	pub furniture: FurnitureNode,
}

/// Free-standing small box (not bed-adjacent).
#[derive(Debug, Clone, PartialEq)]
pub struct SmallBedroomFurnitureFill {
	pub label: LabelNode,
	pub furniture: FurnitureNode,
}

/// Free-standing wardrobe (not inside a closet cell).
#[derive(Debug, Clone, PartialEq)]
pub struct WardrobeFill {
	pub label: LabelNode,
	pub furniture: FurnitureNode,
}

/// Free-standing dresser.
#[derive(Debug, Clone, PartialEq)]
pub struct DresserFill {
	pub label: LabelNode,
	pub furniture: FurnitureNode,
}

/// Mid-size free furniture for roomy hosts.
#[derive(Debug, Clone, PartialEq)]
pub struct BedroomFurnitureFill {
	pub label: LabelNode,
	pub furniture: FurnitureNode,
}

/// Shallow closet residual (walled room only — storage furniture is free-standing).
#[derive(Debug, Clone, PartialEq)]
pub struct ClosetFill {
	pub room: WalledRoomFill,
}

impl std::ops::Deref for ClosetFill {
	type Target = WalledRoomFill;

	fn deref(&self) -> &Self::Target {
		&self.room
	}
}

/// Walk-in closet residual (larger mins / area target than [`ClosetFill`]).
#[derive(Debug, Clone, PartialEq)]
pub struct WalkInClosetFill {
	pub room: WalledRoomFill,
}

impl std::ops::Deref for WalkInClosetFill {
	type Target = WalledRoomFill;

	fn deref(&self) -> &Self::Target {
		&self.room
	}
}

/// Ensuite residual (walled room + door); interior composed via [`ResidentialBathroom`].
#[derive(Debug, Clone, PartialEq)]
pub struct EnsuiteFill {
	pub room: WalledRoomFill,
}

impl std::ops::Deref for EnsuiteFill {
	type Target = WalledRoomFill;

	fn deref(&self) -> &Self::Target {
		&self.room
	}
}

impl CommonBedroom {
	pub fn from_plan(plan: CommonBedroomPlan, confines: &Confines) -> Self {
		let style = plan.parameterized.style;
		let host = &confines.bounds;
		let beds = plan
			.packed
			.beds
			.iter()
			.map(|aabb| {
				let fill =
					furniture_fill(style, "Bed", aabb, host, confines.roll, FurnitureNode::bed);
				BedFill { label: fill.label, furniture: fill.furniture }
			})
			.collect();
		let nightstands = plan
			.packed
			.nightstands
			.iter()
			.map(|aabb| {
				let fill = furniture_fill(
					style,
					"Nightstand",
					aabb,
					host,
					confines.roll,
					FurnitureNode::nightstand,
				);
				NightstandFill { label: fill.label, furniture: fill.furniture }
			})
			.collect();
		let small_bedroom_furniture = plan
			.packed
			.small_bedroom_furniture
			.iter()
			.map(|aabb| {
				let fill = furniture_fill(
					style,
					"SmallBedroomFurniture",
					aabb,
					host,
					confines.roll,
					FurnitureNode::chest,
				);
				SmallBedroomFurnitureFill { label: fill.label, furniture: fill.furniture }
			})
			.collect();
		let wardrobes = plan
			.packed
			.wardrobes
			.iter()
			.map(|aabb| {
				let fill = furniture_fill(
					style,
					"Wardrobe",
					aabb,
					host,
					confines.roll,
					FurnitureNode::wardrobe,
				);
				WardrobeFill { label: fill.label, furniture: fill.furniture }
			})
			.collect();
		let dressers = plan
			.packed
			.dressers
			.iter()
			.map(|aabb| {
				let fill = furniture_fill(
					style,
					"Dresser",
					aabb,
					host,
					confines.roll,
					FurnitureNode::dresser,
				);
				DresserFill { label: fill.label, furniture: fill.furniture }
			})
			.collect();
		let bedroom_furniture = plan
			.packed
			.bedroom_furniture
			.iter()
			.map(|aabb| {
				let fill = furniture_fill(
					style,
					"BedroomFurniture",
					aabb,
					host,
					confines.roll,
					FurnitureNode::bedroom_furniture,
				);
				BedroomFurnitureFill { label: fill.label, furniture: fill.furniture }
			})
			.collect();

		let mut closet_walls = Vec::new();
		let closets = plan
			.packed
			.closets
			.iter()
			.map(|p| {
				closet_walls.extend(p.walls.iter().cloned());
				ClosetFill { room: partition_to_walled(p, style, "Closet", confines.roll) }
			})
			.collect();
		let walk_in_closets = plan
			.packed
			.walk_in_closets
			.iter()
			.map(|p| {
				closet_walls.extend(p.walls.iter().cloned());
				WalkInClosetFill {
					room: partition_to_walled(p, style, "WalkInCloset", confines.roll),
				}
			})
			.collect();

		let mut ensuite_walls = Vec::new();
		let ensuites = plan
			.packed
			.ensuites
			.iter()
			.map(|p| {
				ensuite_walls.extend(p.walls.iter().cloned());
				EnsuiteFill { room: partition_to_walled(p, style, "Ensuite", confines.roll) }
			})
			.collect();

		Self {
			room_type: label_filling_aabb(
				LabelStyle::Blue,
				"CommonBedroom",
				&confines.bounds,
				confines.roll,
			),
			beds,
			nightstands,
			small_bedroom_furniture,
			wardrobes,
			dressers,
			bedroom_furniture,
			closet_walls,
			ensuite_walls,
			closets,
			walk_in_closets,
			ensuites,
			ensuite_bathrooms: Vec::new(),
		}
	}

	fn partition_regions(&self, roll: f32) -> Vec<FillRegion> {
		let mut out = Vec::new();
		for c in &self.closets {
			out.push(c.room.to_fill_region(SpaceKind::InternalSpace, roll));
		}
		for c in &self.walk_in_closets {
			out.push(c.room.to_fill_region(SpaceKind::InternalSpace, roll));
		}
		for e in &self.ensuites {
			out.push(e.room.to_fill_region(SpaceKind::InternalSpace, roll));
		}
		out
	}
}

fn partition_to_walled(
	part: &BedroomPartition,
	style: LabelStyle,
	label: &str,
	roll: f32,
) -> WalledRoomFill {
	WalledRoomFill::new(
		part.bounds,
		part.walls.clone(),
		part.door_id.clone(),
		part.door.clone(),
		style,
		label,
		roll,
	)
}

fn compose_ensuite_bathrooms(
	ensuites: &[EnsuiteFill],
	roll: f32,
	noise: NoiseParams,
) -> Vec<ResidentialBathroom> {
	let mut out = Vec::new();
	for (i, ensuite) in ensuites.iter().enumerate() {
		let region = ensuite.room.to_fill_region(SpaceKind::InternalSpace, roll);
		let bath_noise =
			NoiseParams { seed: noise.seed.wrapping_add(i as i32).wrapping_mul(7919), ..noise };
		if let Ok((bath, _)) = ResidentialBathroom::fit_to_confines(&region.confines, bath_noise) {
			out.push(bath);
		}
	}
	out
}

fn finish_fit(
	mut room: CommonBedroom,
	confines: &Confines,
	noise: NoiseParams,
) -> (CommonBedroom, FillableRegions) {
	room.ensuite_bathrooms = compose_ensuite_bathrooms(&room.ensuites, confines.roll, noise);
	let regions =
		FillableRegions { within: room.partition_regions(confines.roll), atop: Vec::new() };
	(room, regions)
}

impl Fit for CommonBedroom {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = CommonBedroomParameterized::sample(confines, noise)?;
		let plan = CommonBedroomPlan::from_parameterized(params, confines, noise)?;
		Ok(finish_fit(Self::from_plan(plan, confines), confines, noise))
	}
}

impl CommonBedroom {
	/// Fit with explicit parameterized knobs (playground `/show bedroom`).
	pub fn fit_with_fill(
		confines: &Confines,
		noise: NoiseParams,
		params: CommonBedroomParameterized,
	) -> Result<(Self, FillableRegions), FitError> {
		let plan = CommonBedroomPlan::from_parameterized(params, confines, noise)?;
		Ok(finish_fit(Self::from_plan(plan, confines), confines, noise))
	}
}

impl BuildingComponents for CommonBedroom {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for wall in &self.closet_walls {
			out.extend(wall.panel_nodes_for_level(level));
		}
		for wall in &self.ensuite_walls {
			out.extend(wall.panel_nodes_for_level(level));
		}
		out
	}

	fn label_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<LabelNode> {
		let mut labels = vec![self.room_type.clone()];
		labels.extend(self.beds.iter().map(|b| b.label.clone()));
		labels.extend(self.nightstands.iter().map(|n| n.label.clone()));
		labels.extend(self.small_bedroom_furniture.iter().map(|s| s.label.clone()));
		labels.extend(self.wardrobes.iter().map(|w| w.label.clone()));
		labels.extend(self.dressers.iter().map(|d| d.label.clone()));
		labels.extend(self.bedroom_furniture.iter().map(|b| b.label.clone()));
		labels.extend(self.closets.iter().map(|c| c.room.label.clone()));
		labels.extend(self.walk_in_closets.iter().map(|c| c.room.label.clone()));
		labels.extend(self.ensuites.iter().map(|e| e.room.label.clone()));
		labels.extend(self.ensuite_bathrooms.iter().map(|b| b.room_type.clone()));
		Layers::from_free(labels)
	}

	fn furniture_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FurnitureNode> {
		let mut out = Layers::new();
		out.extend(Layers::from_free(
			self.beds.iter().map(|b| b.furniture.clone()).collect::<Vec<_>>(),
		));
		out.extend(Layers::from_free(
			self.nightstands.iter().map(|n| n.furniture.clone()).collect::<Vec<_>>(),
		));
		out.extend(Layers::from_free(
			self.small_bedroom_furniture
				.iter()
				.map(|s| s.furniture.clone())
				.collect::<Vec<_>>(),
		));
		out.extend(Layers::from_free(
			self.wardrobes.iter().map(|w| w.furniture.clone()).collect::<Vec<_>>(),
		));
		out.extend(Layers::from_free(
			self.dressers.iter().map(|d| d.furniture.clone()).collect::<Vec<_>>(),
		));
		out.extend(Layers::from_free(
			self.bedroom_furniture.iter().map(|b| b.furniture.clone()).collect::<Vec<_>>(),
		));
		out
	}
}
