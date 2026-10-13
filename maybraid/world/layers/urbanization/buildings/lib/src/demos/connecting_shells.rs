//! Demo: [`ArcTower`] joined to a [`Trazaloid`] by a [`ConnectingHall`].
//!
//! # Joinery note
//!
//! Start from the two openings and connect **backwards**. Prefer endpoints that
//! [`crate::openings::MappedOpening::widened`] past the jambs and
//! [`crate::openings::MappedOpening::raised`] above the lintel — a connector
//! that reads slightly proud of the door looks better than one that stops short.

use bevy_math::Vec3;
use building_components::floors::FloorNode;
use building_components::joints::JointNode;
use building_components::panels::PanelNode;
use building_components::partitions::{PartitionNode, PartitionStyle};
use building_components::{BuildingComponents, Layers};
use lod::scene::LodSceneLevel;

use crate::openings::{MapsOpenings, OpeningId, OpeningLabel, Openings};
use crate::shells::arc_floor::{ArcFloor, ArcFloorSlab};
use crate::shells::arc_tower::{ArcTower, ArcTowerParams};
use crate::shells::connecting_hall::ConnectingHall;
use crate::shells::trazaloid::{Trazaloid, TrazaloidParams, TrazaloidSide, TrazaloidSlab};

/// Shared contract id for the hall join on both shells.
const CONNECT: &str = "connect";

/// Door facing the trazaloid: \(t = 0 → +X\) (arc assets on local +X).
const DOOR_T: f32 = 0.0;

/// Extra meters past each jamb on the arc-tower hall end (15° door is narrow).
const TOWER_OVERRUN_M: f32 = 0.35;
/// Extra meters past each jamb on the trazaloid hall end (reads wider than the door).
const TRAZALOID_OVERRUN_M: f32 = 1.1;
/// Extra meters of tube height above each mapped lintel.
const HALL_HEADER_M: f32 = 0.75;

/// Fixed composition: circular tower west of a trazaloid, linked by a one-kink hall.
#[derive(Debug, Clone, PartialEq)]
pub struct ConnectingShells {
	tower: ArcTower,
	hall: ConnectingHall,
	trazaloid: Trazaloid,
}

impl ConnectingShells {
	pub fn new() -> Self {
		let tower_center = Vec3::new(-14.0, 0.0, 0.0);
		let radius = 4.0;
		let storey_height = 3.0;

		let mut tower_openings = Openings::new();
		for (id, t, label) in [
			("window_w", 0.25, OpeningLabel::Aperture), // −Z
			(CONNECT, DOOR_T, OpeningLabel::Passage),   // +X
			("window_e", 0.5, OpeningLabel::Aperture),  // −X
			("window_n", 0.75, OpeningLabel::Aperture), // +Z
		] {
			let (id, opening) =
				ArcFloor::plan_opening_at_t(id, label, tower_center, radius, storey_height, t);
			tower_openings.insert(id, opening);
		}

		let tower = ArcTower::new(ArcTowerParams {
			// Door on +X sits near x ≈ -10; trazaloid west face at x = -4.
			center_xz: tower_center,
			radius,
			floor_count: 3,
			storey_height,
			openings: tower_openings,
			base_floor: ArcFloorSlab::Solid,
			intermediate_floors: ArcFloorSlab::Solid,
			top_ceiling: ArcFloorSlab::Solid,
			intermediate_floor_hole: 2.24,
			style: PartitionStyle::RoughStonework,
		});

		let footprint = bevy_math::Vec2::new(8.0, 6.0);
		let connect_id = OpeningId::new(CONNECT);
		let trazaloid = Trazaloid::new(TrazaloidParams {
			origin: Vec3::ZERO,
			footprint,
			ridge: bevy_math::Vec2::new(4.0, 3.0),
			lower_height: 3.0,
			upper_height: 2.5,
			band_vertical_offset: 0.35,
			waist_horizontal_offset: 0.25,
			openings: Openings::new().with(
				connect_id.clone(),
				Trazaloid::side_passage_opening(TrazaloidSide::West, footprint, 1.2, 2.1),
			),
			floor: TrazaloidSlab::None,
			ceiling: TrazaloidSlab::Solid,
			style: building_components::panels::PanelStyle::RoughStonework,
			joint_thickness: crate::paneling::panel_complex::DEFAULT_PANEL_THICKNESS,
			face_post_count: 2,
		});

		// Hall tops on the trazaloid end follow the footprint→waist pitch (door clip
		// points lie on that face; Tube stations keep authored `top_middle`).
		let end_tower = tower
			.mapped_opening(0, &connect_id)
			.expect("arc tower ground door")
			.widened(TOWER_OVERRUN_M)
			.raised(HALL_HEADER_M);
		let end_traz = trazaloid
			.mapped_opening(&connect_id)
			.expect("trazaloid west door")
			.widened(TRAZALOID_OVERRUN_M)
			.raised(HALL_HEADER_M);
		let hall = ConnectingHall::rough_stone(end_tower, end_traz);

		Self { tower, hall, trazaloid }
	}

	pub fn tower(&self) -> &ArcTower {
		&self.tower
	}

	pub fn hall(&self) -> &ConnectingHall {
		&self.hall
	}

	pub fn trazaloid(&self) -> &Trazaloid {
		&self.trazaloid
	}
}

impl Default for ConnectingShells {
	fn default() -> Self {
		Self::new()
	}
}

impl BuildingComponents for ConnectingShells {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = self.trazaloid.panel_nodes_for_level(level);
		out.extend(self.hall.panel_nodes_for_level(level));
		out
	}

	fn partition_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartitionNode> {
		self.tower.partition_nodes_for_level(level)
	}

	fn floor_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FloorNode> {
		self.tower.floor_nodes_for_level(level)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = self.trazaloid.joint_nodes_for_level(level);
		out.extend(self.hall.joint_nodes_for_level(level));
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn hall_stations_built_between_shells() -> anyhow::Result<()> {
		let demo = ConnectingShells::new();
		let stations = demo.hall().stations();
		assert!(stations[0].bottom_middle.x < stations[2].bottom_middle.x);
		assert!(demo.hall().midpoint().x.is_finite());
		assert!(!demo.hall().panel_nodes_for_level(LodSceneLevel::High).flatten().is_empty());
		assert_eq!(demo.tower().storeys().len(), 3);
		Ok(())
	}

	#[test]
	fn trazaloid_hall_end_carries_face_pitch() -> anyhow::Result<()> {
		let demo = ConnectingShells::new();
		let (_a, end_b) = demo.hall().endpoints();
		// West door: top should be inset (larger x) relative to bottom.
		let (bl, br, tl, tr) = end_b.endpoint_corners();
		let bottom_x = 0.5 * (bl.x + br.x);
		let top_x = 0.5 * (tl.x + tr.x);
		assert!(top_x > bottom_x + 1e-3);
		let stations = demo.hall().stations();
		let traz_station = stations[2];
		let top = traz_station
			.top_middle
			.ok_or_else(|| anyhow::anyhow!("pitched top_middle missing"))?;
		assert!(
			(top.x - traz_station.bottom_middle.x).abs() > 1e-3,
			"tube station should keep non-vertical top lift"
		);
		Ok(())
	}

	#[test]
	fn tower_hall_end_faces_east_door() -> anyhow::Result<()> {
		let demo = ConnectingShells::new();
		let (end_a, _) = demo.hall().endpoints();
		let (bl, br, ..) = end_a.endpoint_corners();
		let mid = (bl + br) * 0.5;
		// Layer 1 maps the connect opening onto hit 15° sectors around t=0 (+X).
		assert!(mid.x > -11.0, "mid={mid:?}");
		assert!(mid.z.abs() < 1.5, "mid={mid:?}");
		assert!(end_a.orientation.normalize().x > 0.7, "orient={:?}", end_a.orientation);
		assert!(bl.distance(br) > 0.4, "door span too narrow");
		Ok(())
	}

	#[test]
	fn both_ends_widen_past_door_jambs() -> anyhow::Result<()> {
		let demo = ConnectingShells::new();
		let stations = demo.hall().stations();
		// Tower: ~R sin(7.5°) + overrun ≈ 0.52 + 0.35
		assert!(stations[0].bottom_left_width > 0.8, "{}", stations[0].bottom_left_width);
		assert!((stations[0].bottom_left_width - stations[0].bottom_right_width).abs() < 1e-3);
		// Trazaloid: door half 0.6 + overrun 1.1 = 1.7 (must not shrink from L/R swap)
		assert!(
			stations[2].bottom_left_width > 1.5,
			"traz width {} — outward corner order / widen bug?",
			stations[2].bottom_left_width
		);
		assert!((stations[2].bottom_left_width - stations[2].bottom_right_width).abs() < 1e-3);
		Ok(())
	}
}
