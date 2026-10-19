//! Geometric semantic-cell identity. Matches Chico forest / grove grids
//! ([`chico::ForestExtent`], [`chico::grove_id`]) without a Chico dependency.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::Vec3;

use crate::metadata::CellMetadataStore;

/// Square forest cell span in metres on X and Z. Same as
/// `chico::DEFAULT_FOREST_EXTENT_XZ`.
pub const FOREST_EXTENT_XZ: f32 = 1600.0;

/// Grove tile span in metres on X and Z. Same as
/// `chico::DEFAULT_FOREST_GROVE_TILE_XZ`.
pub const GROVE_TILE_XZ: f32 = 100.0;

/// World X/Z of the origin forest cell's min face.
pub const GROVE_ORIGIN: f32 = -FOREST_EXTENT_XZ * 0.5;

/// One forest vegetation layer. Y encoding matches `chico::ForestLayer`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum VegetationLayer {
	Tufts,
	Understory,
	LowerCanopy,
	UpperCanopy,
}

impl VegetationLayer {
	pub const ALL: [Self; 4] =
		[Self::Tufts, Self::Understory, Self::LowerCanopy, Self::UpperCanopy];

	pub fn id_y(self) -> f32 {
		match self {
			Self::Tufts => 0.0,
			Self::Understory => 1.0,
			Self::LowerCanopy => 2.0,
			Self::UpperCanopy => 3.0,
		}
	}

	pub fn from_id_y(y: f32) -> Option<Self> {
		match y.floor() as i32 {
			0 => Some(Self::Tufts),
			1 => Some(Self::Understory),
			2 => Some(Self::LowerCanopy),
			3 => Some(Self::UpperCanopy),
			_ => None,
		}
	}

	pub fn index(self) -> usize {
		match self {
			Self::Tufts => 0,
			Self::Understory => 1,
			Self::LowerCanopy => 2,
			Self::UpperCanopy => 3,
		}
	}
}

/// Origin-centered 1600 m forest cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ForestCellId {
	pub ix: i32,
	pub iz: i32,
}

impl ForestCellId {
	pub fn containing(position: Vec3) -> Self {
		let h = FOREST_EXTENT_XZ * 0.5;
		Self {
			ix: ((position.x + h) / FOREST_EXTENT_XZ).floor() as i32,
			iz: ((position.z + h) / FOREST_EXTENT_XZ).floor() as i32,
		}
	}

	pub fn min_xz(self) -> (f32, f32) {
		let h = FOREST_EXTENT_XZ * 0.5;
		(self.ix as f32 * FOREST_EXTENT_XZ - h, self.iz as f32 * FOREST_EXTENT_XZ - h)
	}

	pub fn aabb(self) -> Aabb3d {
		let (x, z) = self.min_xz();
		Aabb3d::from_min_max(
			Vec3::new(x, 0.0, z),
			Vec3::new(x + FOREST_EXTENT_XZ, 1.0, z + FOREST_EXTENT_XZ),
		)
	}

	pub fn contains_xz(self, position: Vec3) -> bool {
		let (x, z) = self.min_xz();
		position.x >= x
			&& position.x < x + FOREST_EXTENT_XZ
			&& position.z >= z
			&& position.z < z + FOREST_EXTENT_XZ
	}
}

/// World-aligned 100 m grove tile, locked to the forest origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GroveTileId {
	pub ix: i32,
	pub iz: i32,
}

impl GroveTileId {
	pub fn containing(position: Vec3) -> Self {
		Self {
			ix: ((position.x - GROVE_ORIGIN) / GROVE_TILE_XZ).floor() as i32,
			iz: ((position.z - GROVE_ORIGIN) / GROVE_TILE_XZ).floor() as i32,
		}
	}

	pub fn min_xz(self) -> (f32, f32) {
		(
			GROVE_ORIGIN + self.ix as f32 * GROVE_TILE_XZ,
			GROVE_ORIGIN + self.iz as f32 * GROVE_TILE_XZ,
		)
	}

	pub fn aabb(self) -> Aabb3d {
		let (x, z) = self.min_xz();
		Aabb3d::from_min_max(
			Vec3::new(x, 0.0, z),
			Vec3::new(x + GROVE_TILE_XZ, 1.0, z + GROVE_TILE_XZ),
		)
	}

	pub fn forest(self) -> ForestCellId {
		let (x, z) = self.min_xz();
		ForestCellId::containing(Vec3::new(x + GROVE_TILE_XZ * 0.5, 0.0, z + GROVE_TILE_XZ * 0.5))
	}

	pub fn contains_xz(self, position: Vec3) -> bool {
		let (x, z) = self.min_xz();
		position.x >= x
			&& position.x < x + GROVE_TILE_XZ
			&& position.z >= z
			&& position.z < z + GROVE_TILE_XZ
	}
}

/// One forest layer on a grove tile. Same identity as `chico::grove_id`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GroveLayerId {
	pub tile: GroveTileId,
	pub layer: VegetationLayer,
}

impl GroveLayerId {
	pub fn aabb(self) -> Aabb3d {
		let (x, z) = self.tile.min_xz();
		let y = self.layer.id_y();
		Aabb3d::from_min_max(
			Vec3::new(x, y, z),
			Vec3::new(x + GROVE_TILE_XZ, y + 1.0, z + GROVE_TILE_XZ),
		)
	}
}

/// World-aligned planting sub-cell whose center is owned by `tile`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlantingCellId {
	pub tile: GroveTileId,
	pub layer: VegetationLayer,
	pub cx: i32,
	pub cz: i32,
}

impl PlantingCellId {
	pub fn containing(
		tile: GroveTileId,
		layer: VegetationLayer,
		extent_xz: f32,
		position: Vec3,
	) -> Self {
		let step = extent_xz.max(0.1);
		Self {
			tile,
			layer,
			cx: (position.x / step).floor() as i32,
			cz: (position.z / step).floor() as i32,
		}
	}

	pub fn aabb(self, extent_xz: f32) -> Aabb3d {
		let step = extent_xz.max(0.1);
		let y = self.layer.id_y();
		Aabb3d::from_min_max(
			Vec3::new(self.cx as f32 * step, y, self.cz as f32 * step),
			Vec3::new((self.cx as f32 + 1.0) * step, y + 1.0, (self.cz as f32 + 1.0) * step),
		)
	}
}

/// Publisher-assigned urban leaf / development id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UrbanCellId(pub u64);

/// Cache key for one generated semantic cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SemanticCellKey {
	Forest(ForestCellId),
	GroveLayer(GroveLayerId),
	Planting(PlantingCellId),
	Urban(UrbanCellId),
}

impl SemanticCellKey {
	pub fn default_bounds(self) -> Aabb3d {
		match self {
			Self::Forest(id) => id.aabb(),
			Self::GroveLayer(id) => id.aabb(),
			Self::Planting(id) => id.aabb(GROVE_TILE_XZ),
			Self::Urban(_) => Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE),
		}
	}

	pub fn forest(self) -> Option<ForestCellId> {
		match self {
			Self::Forest(id) => Some(id),
			Self::GroveLayer(id) => Some(id.tile.forest()),
			Self::Planting(id) => Some(id.tile.forest()),
			Self::Urban(_) => None,
		}
	}

	pub fn grove(self) -> Option<GroveTileId> {
		match self {
			Self::Forest(_) | Self::Urban(_) => None,
			Self::GroveLayer(id) => Some(id.tile),
			Self::Planting(id) => Some(id.tile),
		}
	}
}

/// Geometric occupancy used for enter/exit (layers stack; the actor occupies a tile).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum OccupiedRegion {
	Forest(ForestCellId),
	Grove(GroveTileId),
	Planting(PlantingCellId),
	Urban(UrbanCellId),
}

/// Rule-timer key. Grove-layer is used only when a rule names a layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum OccupancyMatchKey {
	Forest(ForestCellId),
	Grove(GroveTileId),
	GroveLayer(GroveLayerId),
	Planting(PlantingCellId),
	Urban(UrbanCellId),
}

impl OccupancyMatchKey {
	pub fn occupies(self, identity: &OccupancyIdentity) -> bool {
		match self {
			Self::Forest(id) => identity.forest == id,
			Self::Grove(id) => identity.grove == id,
			Self::GroveLayer(id) => identity.grove == id.tile,
			Self::Planting(id) => identity.planting == Some(id),
			Self::Urban(id) => identity.urban == Some(id),
		}
	}

	pub fn region(self) -> OccupiedRegion {
		match self {
			Self::Forest(id) => OccupiedRegion::Forest(id),
			Self::Grove(id) | Self::GroveLayer(GroveLayerId { tile: id, .. }) => {
				OccupiedRegion::Grove(id)
			}
			Self::Planting(id) => OccupiedRegion::Planting(id),
			Self::Urban(id) => OccupiedRegion::Urban(id),
		}
	}
}

/// Actor's current semantic occupancy. Derived from position + cached metadata.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OccupancyIdentity {
	pub position: Vec3,
	pub forest: ForestCellId,
	pub grove: GroveTileId,
	pub planting: Option<PlantingCellId>,
	pub urban: Option<UrbanCellId>,
}

impl OccupancyIdentity {
	/// Forest and grove tiles are always available from geometry.
	pub fn from_position(position: Vec3) -> Self {
		Self {
			position,
			forest: ForestCellId::containing(position),
			grove: GroveTileId::containing(position),
			planting: None,
			urban: None,
		}
	}

	/// Enrich planting / urban keys from published metadata. Never reads meshes.
	pub fn resolve(position: Vec3, store: &CellMetadataStore) -> Self {
		let mut identity = Self::from_position(position);
		identity.planting = store.planting_at(identity.grove, position);
		identity.urban = store.urban_at(position);
		identity
	}

	pub fn regions(self) -> [Option<OccupiedRegion>; 4] {
		[
			Some(OccupiedRegion::Forest(self.forest)),
			Some(OccupiedRegion::Grove(self.grove)),
			self.planting.map(OccupiedRegion::Planting),
			self.urban.map(OccupiedRegion::Urban),
		]
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn forest_and_grove_match_chico_origin_grid() -> anyhow::Result<()> {
		assert_eq!(ForestCellId::containing(Vec3::ZERO), ForestCellId { ix: 0, iz: 0 });
		assert_eq!(ForestCellId::containing(Vec3::new(799.9, 0.0, 0.0)).ix, 0);
		assert_eq!(ForestCellId::containing(Vec3::new(800.0, 0.0, 0.0)).ix, 1);
		assert_eq!(ForestCellId::containing(Vec3::new(-800.0, 0.0, 0.0)).ix, 0);
		assert_eq!(ForestCellId::containing(Vec3::new(-800.1, 0.0, 0.0)).ix, -1);
		assert_eq!(GroveTileId::containing(Vec3::ZERO), GroveTileId { ix: 8, iz: 8 });
		assert!(GroveTileId::containing(Vec3::ZERO).contains_xz(Vec3::ZERO));
		assert!(!GroveTileId::containing(Vec3::ZERO).contains_xz(Vec3::new(100.0, 0.0, 0.0)));
		Ok(())
	}

	#[test]
	fn grove_tile_belongs_to_its_forest() -> anyhow::Result<()> {
		let grove = GroveTileId::containing(Vec3::ZERO);
		assert_eq!(grove.forest(), ForestCellId { ix: 0, iz: 0 });
		let east = GroveTileId::containing(Vec3::new(800.0, 0.0, 0.0));
		assert_eq!(east.forest(), ForestCellId { ix: 1, iz: 0 });
		Ok(())
	}

	#[test]
	fn half_open_faces_do_not_double_occupy() -> anyhow::Result<()> {
		let on_face = Vec3::new(800.0, 0.0, 0.0);
		let left = OccupancyIdentity::from_position(Vec3::new(799.9, 0.0, 0.0));
		let right = OccupancyIdentity::from_position(on_face);
		assert_ne!(left.forest, right.forest);
		assert_ne!(left.grove, right.grove);
		Ok(())
	}
}
