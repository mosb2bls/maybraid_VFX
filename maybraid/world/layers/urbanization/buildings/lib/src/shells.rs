//! Building envelope shells composed over paneling primitives.

pub mod arc_floor;
pub mod arc_tower;
pub mod circ_ring_floor;
pub mod connecting_hall;
pub mod i_floor;
pub mod ortho;
pub mod pitched_rectangular_roof;
pub mod rect_floor;
pub mod rect_ring_floor;
pub mod rectangular_pitched_roof_complex;
pub mod rounded_rect_floor;
pub mod trazaloid;

pub use arc_floor::{ArcFloor, ArcFloorParams, ArcFloorSlab};
pub use arc_tower::{ArcTower, ArcTowerParams};
pub use circ_ring_floor::{CircRingFloor, CircRingFloorParams, CircRingFloorSlab};
pub use connecting_hall::ConnectingHall;
pub use i_floor::{IFloor, IFloorParams, IFloorPlanRect, IFloorSlab};
pub use pitched_rectangular_roof::{PitchedRoof, PitchedRoofParams, RoofHalf};
pub use rect_floor::{RectFloor, RectFloorParams, RectFloorSide, RectFloorSlab};
pub use rect_ring_floor::{
	RectRingFloor, RectRingFloorParams, RectRingFloorSide, RectRingFloorSlab,
};
pub use rectangular_pitched_roof_complex::{
	EndCap, Overhang, RectangularPitchedRoofComplex, RectangularPitchedRoofComplexParams,
	RidgeJunction, ValleySegment,
};
pub use rounded_rect_floor::{
	RoundedRectCorner, RoundedRectFloor, RoundedRectFloorParams, RoundedRectFloorSide,
	RoundedRectFloorSlab,
};
pub use trazaloid::{Trazaloid, TrazaloidParams, TrazaloidSide, TrazaloidSlab};
