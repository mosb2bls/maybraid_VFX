//! Packed spaces inside a [`super::LivableApartment`].

use building_components::furniture::FurnitureNode;
use building_components::joints::JointNode;
use building_components::labels::LabelNode;
use building_components::panels::PanelNode;
use building_components::{BuildingComponents, Layers};
use lod::scene::LodSceneLevel;

use crate::fit::Confines;
use crate::usage_areas::common_bedroom::CommonBedroom;
use crate::usage_areas::livable_quarters::{
	DiningRoom, Kitchen, LivingRoom, ResidentialBathroom, ResidentialHalfBathroom, SittingRoom,
	Study,
};

/// One packed space inside an apartment.
#[derive(Debug, Clone, PartialEq)]
pub enum ApartmentRoom {
	Entryway {
		label: LabelNode,
		confines: Confines,
	},
	HouseholdCloset {
		label: LabelNode,
		confines: Confines,
	},
	Bedroom(CommonBedroom),
	Living(LivingRoom),
	Kitchen(Kitchen),
	Dining(DiningRoom),
	Bathroom(ResidentialBathroom),
	HalfBath(ResidentialHalfBathroom),
	Sitting(SittingRoom),
	Study(Study),
	/// Open hall band from a rectangular livable area (no furniture).
	OpenHall {
		label: LabelNode,
		confines: Confines,
	},
}

impl ApartmentRoom {
	pub(crate) fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		match self {
			Self::Entryway { .. } | Self::HouseholdCloset { .. } | Self::OpenHall { .. } => {
				Layers::new()
			}
			Self::Bedroom(r) => r.panel_nodes_for_level(level),
			Self::Living(r) => r.panel_nodes_for_level(level),
			Self::Kitchen(r) => r.panel_nodes_for_level(level),
			Self::Dining(r) => r.panel_nodes_for_level(level),
			Self::Bathroom(r) => r.panel_nodes_for_level(level),
			Self::HalfBath(r) => r.panel_nodes_for_level(level),
			Self::Sitting(r) => r.panel_nodes_for_level(level),
			Self::Study(r) => r.panel_nodes_for_level(level),
		}
	}

	pub(crate) fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		match self {
			Self::Entryway { .. } | Self::HouseholdCloset { .. } | Self::OpenHall { .. } => {
				Layers::new()
			}
			Self::Bedroom(r) => r.joint_nodes_for_level(level),
			Self::Living(r) => r.joint_nodes_for_level(level),
			Self::Kitchen(r) => r.joint_nodes_for_level(level),
			Self::Dining(r) => r.joint_nodes_for_level(level),
			Self::Bathroom(r) => r.joint_nodes_for_level(level),
			Self::HalfBath(r) => r.joint_nodes_for_level(level),
			Self::Sitting(r) => r.joint_nodes_for_level(level),
			Self::Study(r) => r.joint_nodes_for_level(level),
		}
	}

	pub(crate) fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		match self {
			Self::Entryway { label, .. }
			| Self::HouseholdCloset { label, .. }
			| Self::OpenHall { label, .. } => {
				let mut out = Layers::new();
				out.push_free(label.clone());
				out
			}
			Self::Bedroom(r) => r.label_nodes_for_level(level),
			Self::Living(r) => r.label_nodes_for_level(level),
			Self::Kitchen(r) => r.label_nodes_for_level(level),
			Self::Dining(r) => r.label_nodes_for_level(level),
			Self::Bathroom(r) => r.label_nodes_for_level(level),
			Self::HalfBath(r) => r.label_nodes_for_level(level),
			Self::Sitting(r) => r.label_nodes_for_level(level),
			Self::Study(r) => r.label_nodes_for_level(level),
		}
	}

	pub(crate) fn furniture_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FurnitureNode> {
		match self {
			Self::Entryway { .. } | Self::HouseholdCloset { .. } | Self::OpenHall { .. } => {
				Layers::new()
			}
			Self::Bedroom(r) => r.furniture_nodes_for_level(level),
			Self::Living(r) => r.furniture_nodes_for_level(level),
			Self::Kitchen(r) => r.furniture_nodes_for_level(level),
			Self::Dining(r) => r.furniture_nodes_for_level(level),
			Self::Bathroom(r) => r.furniture_nodes_for_level(level),
			Self::HalfBath(r) => r.furniture_nodes_for_level(level),
			Self::Sitting(r) => r.furniture_nodes_for_level(level),
			Self::Study(r) => r.furniture_nodes_for_level(level),
		}
	}

	pub(crate) fn furniture_usage_nodes_for_level(
		&self,
		level: LodSceneLevel,
	) -> Layers<building_components::FurnitureUsageNode> {
		match self {
			Self::Entryway { .. } | Self::HouseholdCloset { .. } | Self::OpenHall { .. } => {
				Layers::new()
			}
			Self::Bedroom(r) => r.furniture_usage_nodes_for_level(level),
			Self::Living(r) => r.furniture_usage_nodes_for_level(level),
			Self::Kitchen(r) => r.furniture_usage_nodes_for_level(level),
			Self::Dining(r) => r.furniture_usage_nodes_for_level(level),
			Self::Bathroom(r) => r.furniture_usage_nodes_for_level(level),
			Self::HalfBath(r) => r.furniture_usage_nodes_for_level(level),
			Self::Sitting(r) => r.furniture_usage_nodes_for_level(level),
			Self::Study(r) => r.furniture_usage_nodes_for_level(level),
		}
	}

	pub(crate) fn is_closed(&self) -> bool {
		matches!(self, Self::Bedroom(_) | Self::Bathroom(_) | Self::HalfBath(_) | Self::Study(_))
	}

	pub(crate) fn is_open_circ(&self) -> bool {
		matches!(
			self,
			Self::Entryway { .. }
				| Self::OpenHall { .. }
				| Self::Living(_)
				| Self::Kitchen(_)
				| Self::Dining(_)
				| Self::Sitting(_)
		)
	}
}
