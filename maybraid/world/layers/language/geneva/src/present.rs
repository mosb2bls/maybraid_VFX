//! Debug overlay of tile bounds and names, read from storage. Not a mesh stream.

use std::collections::HashSet;

use bevy::math::bounding::Aabb3d;
use bevy::math::{Rect, Vec2};
use bevy::prelude::{Local, Res, ResMut, Resource};
use lod::hcsg::{Busy, HcsgStorage};

use crate::hcsg::{naming_revision, read_nearby, LanguageWindow};
use crate::key::NameKey;
use crate::named::{NameEntry, NameSource, Named, Regions};
use crate::places::LanguageGround;
use crate::tiles::{window_tiles, LargeTile};

/// Tile bounds and names for a debug overlay.
///
/// `epoch` advances whenever the contents change, so readers can skip
/// rebuilds while it holds.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct LanguageOverlay {
	pub epoch: u64,
	pub large_tiles: Vec<LargeTileOverlay>,
	pub names: Vec<NamedOverlay>,
}

/// One large tile in the overlay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LargeTileOverlay {
	pub ix: i32,
	pub iz: i32,
	pub language_count: usize,
	pub small_count: usize,
}

/// One name in the overlay.
#[derive(Clone, Debug, PartialEq)]
pub struct NamedOverlay {
	pub key: NameKey,
	pub surface: String,
	pub english: Vec<String>,
	pub provisional: bool,
	pub xz: Vec2,
	pub extent: Rect,
}

impl NamedOverlay {
	fn of(entry: &NameEntry) -> Self {
		Self {
			key: entry.key,
			surface: entry.name.surface.clone(),
			english: entry.name.english.clone(),
			provisional: false,
			xz: entry.xz,
			extent: entry.extent,
		}
	}
}

/// What the overlay last showed: the windows and the stores' membership.
#[derive(Clone, PartialEq)]
pub(crate) struct Shown {
	boxes: Vec<Aabb3d>,
	naming: Option<Aabb3d>,
	revision: u64,
}

/// Rebuilds the overlay from storage when the windows or a store it reads
/// changed; `W` is the ground the names were generated over.
pub(crate) fn present_language_overlay<W: LanguageGround>(
	window: Res<LanguageWindow>,
	storage: Res<HcsgStorage>,
	mut overlay: ResMut<LanguageOverlay>,
	mut shown: Local<Option<Shown>>,
) {
	if window.boxes.is_empty() {
		if shown.take().is_some() {
			*overlay = LanguageOverlay { epoch: overlay.epoch + 1, ..LanguageOverlay::default() };
		}
		return;
	}
	let Ok(revision) = revision::<W>(&storage) else {
		return;
	};
	let next = Shown { boxes: window.boxes.clone(), naming: window.naming_box(), revision };
	if shown.as_ref() == Some(&next) {
		return;
	}
	let Ok((large_tiles, names)) = collect::<W>(&storage, &next) else {
		return;
	};
	*overlay = LanguageOverlay { epoch: overlay.epoch + 1, large_tiles, names };
	*shown = Some(next);
}

/// Latest membership change among the stores the overlay reads.
pub(crate) fn revision<W: LanguageGround>(storage: &HcsgStorage) -> Result<u64, Busy> {
	let tiles = storage.try_membership_revision::<LargeTile>()?;
	let regions = storage.try_membership_revision::<Named<Regions>>()?;
	Ok(tiles.max(regions).max(naming_revision::<W>(storage)?))
}

/// The names the naming window reaches, once per key.
pub(crate) struct Nearby<'a> {
	storage: &'a HcsgStorage,
	region: Aabb3d,
	keys: HashSet<NameKey>,
	names: Vec<NamedOverlay>,
}

impl Nearby<'_> {
	/// Every name of source `S` whose extent or anchor the window reaches.
	pub(crate) fn read<S: NameSource>(&mut self) -> Result<(), Busy> {
		let reach = Rect::from_corners(
			Vec2::new(self.region.min.x, self.region.min.z),
			Vec2::new(self.region.max.x, self.region.max.z),
		);
		for id in self.storage.try_overlapping::<Named<S>>(self.region)? {
			let Some(entry) = self.storage.try_entry::<Named<S>>(id)? else {
				continue;
			};
			for name in &entry.value.names {
				let reached = !reach.intersect(name.extent).is_empty() || reach.contains(name.xz);
				if reached && self.keys.insert(name.key) {
					self.names.push(NamedOverlay::of(name));
				}
			}
		}
		Ok(())
	}
}

/// The tiles and region names under the tile window, then every name the
/// naming window reaches.
fn collect<W: LanguageGround>(
	storage: &HcsgStorage,
	shown: &Shown,
) -> Result<(Vec<LargeTileOverlay>, Vec<NamedOverlay>), Busy> {
	let mut large_tiles = Vec::new();
	let mut names = Vec::new();
	for (ix, iz) in window_tiles(&shown.boxes) {
		let id = LargeTile::id(ix, iz);
		if let Some(tile) = storage.try_entry::<LargeTile>(id)? {
			large_tiles.push(LargeTileOverlay {
				ix,
				iz,
				language_count: tile.value.languages.len(),
				small_count: tile.value.small.len(),
			});
		}
		if let Some(named) = storage.try_entry::<Named<Regions>>(id)? {
			names.extend(named.value.names.iter().map(NamedOverlay::of));
		}
	}
	let Some(region) = shown.naming else {
		return Ok((large_tiles, names));
	};
	let keys = names.iter().map(|name| name.key).collect();
	let mut nearby = Nearby { storage, region, keys, names };
	read_nearby::<W>(&mut nearby)?;
	Ok((large_tiles, nearby.names))
}
