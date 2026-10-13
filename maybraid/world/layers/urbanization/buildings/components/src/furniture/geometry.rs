//! Continuous furniture / fixture forms (placeholder kit).

use bevy::prelude::Color;

/// Furniture or bathroom-fixture form filled by a wireframe box for now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FurnitureGeometry {
	#[default]
	Bed,
	Wardrobe,
	Dresser,
	Nightstand,
	BedroomFurniture,
	Toilet,
	Chair,
	Table,
	Chest,
	Counter,
	Shelf,
	Range,
	Basin,
	Faucet,
	FoodDisplay,
	Fridge,
	Fruit,
	Bread,
	Cookware,
	/// Floor-to-ceiling ornamental screen (slat partition).
	Partition,
}

impl FurnitureGeometry {
	/// Furniture playground slot boxes (cyan = counter, gold = chair, …).
	///
	/// | Kind | Color | sRGBA |
	/// |---|---|---|
	/// | Bed | blue | `0.20, 0.55, 1.00, 1.0` |
	/// | Wardrobe | orange | `1.00, 0.55, 0.12, 1.0` |
	/// | Dresser | coral | `1.00, 0.38, 0.28, 1.0` |
	/// | Nightstand | green | `0.20, 1.00, 0.38, 1.0` |
	/// | BedroomFurniture | lime | `0.55, 1.00, 0.20, 1.0` |
	/// | Toilet | white | `0.95, 0.95, 1.00, 1.0` |
	/// | Chair | gold | `1.00, 0.78, 0.08, 1.0` |
	/// | Table | amber | `0.82, 0.58, 0.22, 1.0` |
	/// | Chest | magenta | `0.88, 0.28, 1.00, 1.0` |
	/// | Counter | cyan | `0.08, 0.95, 1.00, 1.0` |
	/// | Shelf | brown | `0.62, 0.40, 0.22, 1.0` |
	/// | Range | charcoal | `0.25, 0.22, 0.28, 1.0` |
	/// | Basin | pale | `0.75, 0.88, 0.92, 1.0` |
	/// | Faucet | silver | `0.72, 0.74, 0.78, 1.0` |
	/// | FoodDisplay | pink | `1.00, 0.42, 0.62, 1.0` |
	/// | Fridge | ice | `0.55, 0.82, 0.95, 1.0` |
	/// | Fruit | red | `0.95, 0.22, 0.18, 1.0` |
	/// | Bread | wheat | `0.90, 0.72, 0.32, 1.0` |
	/// | Cookware | iron | `0.35, 0.38, 0.42, 1.0` |
	/// | Partition | sage | `0.58, 0.52, 0.38, 1.0` |
	pub fn wireframe_color(self) -> Color {
		match self {
			Self::Bed => Color::srgba(0.20, 0.55, 1.00, 1.0),
			Self::Wardrobe => Color::srgba(1.00, 0.55, 0.12, 1.0),
			Self::Dresser => Color::srgba(1.00, 0.38, 0.28, 1.0),
			Self::Nightstand => Color::srgba(0.20, 1.00, 0.38, 1.0),
			Self::BedroomFurniture => Color::srgba(0.55, 1.00, 0.20, 1.0),
			Self::Toilet => Color::srgba(0.95, 0.95, 1.00, 1.0),
			Self::Chair => Color::srgba(1.00, 0.78, 0.08, 1.0),
			Self::Table => Color::srgba(0.82, 0.58, 0.22, 1.0),
			Self::Chest => Color::srgba(0.88, 0.28, 1.00, 1.0),
			Self::Counter => Color::srgba(0.08, 0.95, 1.00, 1.0),
			Self::Shelf => Color::srgba(0.62, 0.40, 0.22, 1.0),
			Self::Range => Color::srgba(0.25, 0.22, 0.28, 1.0),
			Self::Basin => Color::srgba(0.75, 0.88, 0.92, 1.0),
			Self::Faucet => Color::srgba(0.72, 0.74, 0.78, 1.0),
			Self::FoodDisplay => Color::srgba(1.00, 0.42, 0.62, 1.0),
			Self::Fridge => Color::srgba(0.55, 0.82, 0.95, 1.0),
			Self::Fruit => Color::srgba(0.95, 0.22, 0.18, 1.0),
			Self::Bread => Color::srgba(0.90, 0.72, 0.32, 1.0),
			Self::Cookware => Color::srgba(0.35, 0.38, 0.42, 1.0),
			Self::Partition => Color::srgba(0.58, 0.52, 0.38, 1.0),
		}
	}

	/// Discriminant mixed into [`super::FurnitureNode::finish_seed`].
	pub const fn finish_salt(self) -> u64 {
		match self {
			Self::Bed => 0x6265_6400,
			Self::Wardrobe => 0x7761_7264,
			Self::Dresser => 0x6472_6573,
			Self::Nightstand => 0x6e69_6768,
			Self::BedroomFurniture => 0x6265_6472,
			Self::Toilet => 0x746f_696c,
			Self::Chair => 0x6368_6169,
			Self::Table => 0x7461_626c,
			Self::Chest => 0x6368_6573,
			Self::Counter => 0x636f_756e,
			Self::Shelf => 0x7368_6c66,
			Self::Range => 0x7261_6e67,
			Self::Basin => 0x6261_7369,
			Self::Faucet => 0x6661_7563,
			Self::FoodDisplay => 0x6664_7370,
			Self::Fridge => 0x6672_6964,
			Self::Fruit => 0x6672_7574,
			Self::Bread => 0x6272_6564,
			Self::Cookware => 0x636f_6f6b,
			Self::Partition => 0x7074_746e,
		}
	}

	/// All kinds (for wireframe material registration).
	pub const ALL: [Self; 20] = [
		Self::Bed,
		Self::Wardrobe,
		Self::Dresser,
		Self::Nightstand,
		Self::BedroomFurniture,
		Self::Toilet,
		Self::Chair,
		Self::Table,
		Self::Chest,
		Self::Counter,
		Self::Shelf,
		Self::Range,
		Self::Basin,
		Self::Faucet,
		Self::FoodDisplay,
		Self::Fridge,
		Self::Fruit,
		Self::Bread,
		Self::Cookware,
		Self::Partition,
	];
}
