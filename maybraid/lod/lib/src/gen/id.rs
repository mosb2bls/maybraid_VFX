use bevy::math::bounding::Aabb3d;
use core::cmp::Ordering;

/// Spatial cell bounds for procedural origin ids.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell(pub Aabb3d);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OriginCell(pub Cell);

impl Eq for Cell {}

impl core::hash::Hash for Cell {
	fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
		self.0.min.x.to_bits().hash(state);
		self.0.min.y.to_bits().hash(state);
		self.0.min.z.to_bits().hash(state);
		self.0.max.x.to_bits().hash(state);
		self.0.max.y.to_bits().hash(state);
		self.0.max.z.to_bits().hash(state);
	}
}

impl PartialOrd for Cell {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for Cell {
	fn cmp(&self, other: &Self) -> Ordering {
		fn key(c: &Cell) -> [u32; 6] {
			[
				c.0.min.x.to_bits(),
				c.0.min.y.to_bits(),
				c.0.min.z.to_bits(),
				c.0.max.x.to_bits(),
				c.0.max.y.to_bits(),
				c.0.max.z.to_bits(),
			]
		}
		key(self).cmp(&key(other))
	}
}

impl PartialOrd for OriginCell {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for OriginCell {
	fn cmp(&self, other: &Self) -> Ordering {
		self.0.cmp(&other.0)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bytes(pub [u8; 32]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Id {
	/// Some entities have a custom ID.
	Bytes(Bytes),

	/// Some entities, particularly procedural ones, are identified by their
	/// origin cell.
	OriginCell(OriginCell),
	/// The universal identifier, used for types that do not vary.
	Universal,
}

impl PartialOrd for Id {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for Id {
	fn cmp(&self, other: &Self) -> Ordering {
		match (self, other) {
			(Self::Bytes(a), Self::Bytes(b)) => a.cmp(b),
			(Self::Bytes(_), _) => Ordering::Less,
			(_, Self::Bytes(_)) => Ordering::Greater,
			(Self::OriginCell(a), Self::OriginCell(b)) => a.cmp(b),
			(Self::OriginCell(_), Self::Universal) => Ordering::Less,
			(Self::Universal, Self::OriginCell(_)) => Ordering::Greater,
			(Self::Universal, Self::Universal) => Ordering::Equal,
		}
	}
}

impl Id {
	pub fn from_cell(bounds: Aabb3d) -> Self {
		Self::OriginCell(OriginCell(Cell(bounds)))
	}

	pub fn origin_cell_bounds(self) -> Option<Aabb3d> {
		match self {
			Self::OriginCell(OriginCell(Cell(bounds))) => Some(bounds),
			Self::Bytes(_) => None,
			Self::Universal => None,
		}
	}
}

/// Ids that originate in the region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OriginalId(pub Id);

impl OriginalId {
	pub fn new(id: Id) -> Self {
		Self(id)
	}

	pub fn universal() -> Self {
		Self(Id::Universal)
	}
}
