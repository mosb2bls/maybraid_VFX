//! `/show` catalog: unit assemblies and the Richmond slot gallery.

pub mod transform;

use bevy::prelude::*;
use building_components::FurnitureGeometry;
use clap::{Args, Subcommand};

use crate::preview::{PreviewConfig, PreviewSubject};

pub use transform::ShowTransform;

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum Show {
	/// Unit bed slot (frame / mattress / covers).
	Bed(UnitShow),
	/// Unit chair slot (legs / seat / +Z back).
	Chair(UnitShow),
	/// Unit cafe table (pedestal or four-leg).
	Table(UnitShow),
	/// Unit chest slot (trunk / lid).
	Chest(UnitShow),
	/// Unit counter slot (footer / volume / top).
	Counter(UnitShow),
	/// Sit-on diner display.
	FoodDisplay(UnitShow),
	/// Sit-on fruit.
	Fruit(UnitShow),
	/// Sit-on bread.
	Bread(UnitShow),
	/// Sit-on pot / skillet / saucepan.
	Cookware(UnitShow),
	/// Sit-on basin.
	Basin(UnitShow),
	/// Deck-mount faucet.
	Faucet(UnitShow),
	/// Wall shelf row.
	Shelf(UnitShow),
	/// Range / stove.
	Range(UnitShow),
	/// Refrigerator.
	Fridge(UnitShow),
	/// Floor-to-ceiling slat partition.
	Partition(UnitShow),
	/// Richmond bedroom / kitchen / living packers plus authored extremes.
	Gallery(GalleryShow),
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct UnitShow {
	/// Palette key. Topology stays fixed.
	#[arg(long, default_value_t = 1)]
	pub seed: u64,
	#[command(flatten)]
	pub transform: ShowTransform,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct GalleryShow {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl Show {
	pub fn react(self, commands: &mut Commands) {
		let (subject, transform) = match self {
			Self::Bed(cmd) => (
				PreviewSubject::Unit { geometry: FurnitureGeometry::Bed, seed: cmd.seed },
				cmd.transform.transform(),
			),
			Self::Chair(cmd) => (
				PreviewSubject::Unit { geometry: FurnitureGeometry::Chair, seed: cmd.seed },
				cmd.transform.transform(),
			),
			Self::Table(cmd) => (
				PreviewSubject::Unit { geometry: FurnitureGeometry::Table, seed: cmd.seed },
				cmd.transform.transform(),
			),
			Self::Chest(cmd) => (
				PreviewSubject::Unit { geometry: FurnitureGeometry::Chest, seed: cmd.seed },
				cmd.transform.transform(),
			),
			Self::Counter(cmd) => (
				PreviewSubject::Unit { geometry: FurnitureGeometry::Counter, seed: cmd.seed },
				cmd.transform.transform(),
			),
			Self::FoodDisplay(cmd) => (
				PreviewSubject::Unit { geometry: FurnitureGeometry::FoodDisplay, seed: cmd.seed },
				cmd.transform.transform(),
			),
			Self::Fruit(cmd) => (
				PreviewSubject::Unit { geometry: FurnitureGeometry::Fruit, seed: cmd.seed },
				cmd.transform.transform(),
			),
			Self::Bread(cmd) => (
				PreviewSubject::Unit { geometry: FurnitureGeometry::Bread, seed: cmd.seed },
				cmd.transform.transform(),
			),
			Self::Cookware(cmd) => (
				PreviewSubject::Unit { geometry: FurnitureGeometry::Cookware, seed: cmd.seed },
				cmd.transform.transform(),
			),
			Self::Basin(cmd) => (
				PreviewSubject::Unit { geometry: FurnitureGeometry::Basin, seed: cmd.seed },
				cmd.transform.transform(),
			),
			Self::Faucet(cmd) => (
				PreviewSubject::Unit { geometry: FurnitureGeometry::Faucet, seed: cmd.seed },
				cmd.transform.transform(),
			),
			Self::Shelf(cmd) => (
				PreviewSubject::Unit { geometry: FurnitureGeometry::Shelf, seed: cmd.seed },
				cmd.transform.transform(),
			),
			Self::Range(cmd) => (
				PreviewSubject::Unit { geometry: FurnitureGeometry::Range, seed: cmd.seed },
				cmd.transform.transform(),
			),
			Self::Fridge(cmd) => (
				PreviewSubject::Unit { geometry: FurnitureGeometry::Fridge, seed: cmd.seed },
				cmd.transform.transform(),
			),
			Self::Partition(cmd) => (
				PreviewSubject::Unit { geometry: FurnitureGeometry::Partition, seed: cmd.seed },
				cmd.transform.transform(),
			),
			Self::Gallery(cmd) => (PreviewSubject::Gallery, cmd.transform.transform()),
		};
		commands.insert_resource(PreviewConfig { subject, transform });
	}
}
