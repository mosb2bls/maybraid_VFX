//! Type-erased discoverable places emitted by Richmond hosts and High usage areas.
//!
//! World composition assigns POI kinds. This crate stays free of `Poi` / mob taxonomy.

use bevy::prelude::Component;
use building_components::LabelNode;
use lod::gen::Id;

/// Richmond-local usage of a presented place. World maps these onto POI kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DiscoverablePlaceLabel {
	Stall,
	Lounge,
	Bedroom,
	Sanctum,
	Storey,
	House,
	Hut,
	Market,
	Highrise,
	Skybridge,
	Tower,
	Room,
}

/// Presented place suitable for bounded semantic discovery.
///
/// Host pins set [`Self::persistent`]. High rooms and stalls do not, so they
/// may disappear with High children.
///
/// [`Self::host`] plus [`Self::local`] is the durable identity when the
/// presenter attached a development id. Rounded coordinates, labels, and ECS
/// entity ids are not durable. Places without `host` stay unnamed-stable:
/// consumers should treat those assignments as provisional.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct DiscoverablePlace {
	pub label: DiscoverablePlaceLabel,
	pub arrival_radius: f32,
	pub salience: Option<f32>,
	pub persistent: bool,
	pub host: Option<Id>,
	pub local: u32,
}

/// Compatibility name for the original Les Halles storey marker.
pub type InteriorArea = DiscoverablePlace;

impl Default for DiscoverablePlace {
	fn default() -> Self {
		Self::host(DiscoverablePlaceLabel::Storey, 8.0, 1.1)
	}
}

impl DiscoverablePlace {
	pub const fn host(label: DiscoverablePlaceLabel, arrival_radius: f32, salience: f32) -> Self {
		Self {
			label,
			arrival_radius,
			salience: Some(salience),
			persistent: true,
			host: None,
			local: 0,
		}
	}

	pub const fn high(label: DiscoverablePlaceLabel, arrival_radius: f32, salience: f32) -> Self {
		Self {
			label,
			arrival_radius,
			salience: Some(salience),
			persistent: false,
			host: None,
			local: 0,
		}
	}

	pub const fn with_identity(mut self, host: Id, local: u32) -> Self {
		self.host = Some(host);
		self.local = local;
		self
	}

	pub fn from_label_node(node: &LabelNode) -> Option<Self> {
		let label = DiscoverablePlaceLabel::from_label_text(&node.text)?;
		let extents = node.geometry.extents();
		let radius = (extents.x.min(extents.z) * 0.35).clamp(1.5, 12.0);
		Some(Self::high(label, radius, label.default_salience()))
	}
}

impl DiscoverablePlaceLabel {
	pub const fn default_salience(self) -> f32 {
		match self {
			Self::Stall | Self::Lounge | Self::Market => 1.25,
			Self::Sanctum | Self::Highrise => 1.2,
			Self::Storey | Self::House | Self::Bedroom => 1.1,
			Self::Hut | Self::Skybridge | Self::Tower | Self::Room => 1.05,
		}
	}

	pub const fn salt(self) -> u64 {
		match self {
			Self::Stall => 0x7374_616c_6c00,
			Self::Lounge => 0x6c6f_756e_6765,
			Self::Bedroom => 0x6265_6472_6f6f,
			Self::Sanctum => 0x7361_6e63_7475,
			Self::Storey => 0x7374_6f72_6579,
			Self::House => 0x686f_7573_6500,
			Self::Hut => 0x6875_7400_0000,
			Self::Market => 0x6d61_726b_6574,
			Self::Highrise => 0x6869_6768_7269,
			Self::Skybridge => 0x736b_7962_7269,
			Self::Tower => 0x746f_7765_7200,
			Self::Room => 0x726f_6f6d_0000,
		}
	}

	/// Room-scale usage labels only. Furniture and circulation text is ignored.
	pub fn from_label_text(text: &str) -> Option<Self> {
		Some(match text {
			"Lounge" => Self::Lounge,
			"KnickKnackStall" | "BitesStall" | "BitesSitdownStall" | "PartsStall" | "MiniMart" => {
				Self::Stall
			}
			"CommonBedroom" => Self::Bedroom,
			"LivingRoom"
			| "SittingRoom"
			| "Kitchen"
			| "DiningRoom"
			| "Study"
			| "EatingArea"
			| "ResidentialBathroom"
			| "ResidentialHalfBathroom"
			| "PublicRestroom"
			| "OpenHall"
			| "Entryway" => Self::Room,
			other if other.starts_with("Hall") => Self::Room,
			_ => return None,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use building_components::{LabelGeometry, LabelStyle, Placement};

	#[test]
	fn room_scale_labels_map_and_furniture_does_not() {
		assert_eq!(
			DiscoverablePlaceLabel::from_label_text("Lounge"),
			Some(DiscoverablePlaceLabel::Lounge)
		);
		assert_eq!(
			DiscoverablePlaceLabel::from_label_text("KnickKnackStall"),
			Some(DiscoverablePlaceLabel::Stall)
		);
		assert_eq!(
			DiscoverablePlaceLabel::from_label_text("MiniMart"),
			Some(DiscoverablePlaceLabel::Stall)
		);
		assert_eq!(
			DiscoverablePlaceLabel::from_label_text("CommonBedroom"),
			Some(DiscoverablePlaceLabel::Bedroom)
		);
		assert_eq!(
			DiscoverablePlaceLabel::from_label_text("Hall3"),
			Some(DiscoverablePlaceLabel::Room)
		);
		assert!(DiscoverablePlaceLabel::from_label_text("Nightstand").is_none());
		assert!(DiscoverablePlaceLabel::from_label_text("BitesCounter").is_none());
		assert!(DiscoverablePlaceLabel::from_label_text("Livable 1").is_none());
	}

	#[test]
	fn label_node_place_is_high_only() {
		let node = LabelNode::new(
			LabelStyle::Gray,
			LabelGeometry::rectangle(bevy::math::Vec3::new(6.0, 3.0, 4.0)),
			"Lounge",
			Placement::default(),
		);
		let place =
			DiscoverablePlace::from_label_node(&node).expect("lounge label should become a place");
		assert_eq!(place.label, DiscoverablePlaceLabel::Lounge);
		assert!(!place.persistent);
		assert!((1.5..=12.0).contains(&place.arrival_radius));
	}
}
