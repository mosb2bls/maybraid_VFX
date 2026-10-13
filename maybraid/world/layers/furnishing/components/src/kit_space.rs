//! Blender \(Z\)-up furniture kits → engine \(Y\)-up slot space.
//!
//! Authored (Blender): most parts \(X,Y \in [-1, 1]\), \(Z \in [0, 1]\). Chair
//! legs are tubes \(X,Y \in [-0.2, 0.2]\), \(Z \in [0, 1]\). Chair back fills
//! authored \(+Y\).
//!
//! Remap \((X,Y,Z)_{\text{Blender}} \mapsto (X,Z,Y)_{\text{engine}}\):
//! - plan \(X\) stays \(X\)
//! - authored \(+Y\) (back / head) becomes engine \(+Z\)
//! - authored \(+Z\) (up) becomes engine \(+Y\)
//!
//! After remap a box kit is \(X,Z \in [-1, 1]\), \(Y \in [0, 1]\) (origin on
//! the floor). The chest latch is front-centroid: \(X,Y \in [-1, 1]\),
//! \(Z \in [0, 1]\) inward from the front face. [`BOX_KIT_TO_UNIT`] maps a
//! floor box into the Richmond unit slot cube \([-0.5, 0.5]^3\). [`slab`] is
//! floor-origin in that cube; [`latch_slab`] is the latch equivalent. Compose
//! `kit_to_unit.compose_child(unit)` before applying the slot placement.

use bevy::math::Vec3;
use building_components::Placement;

/// Authored box kit after remap: plan \(X,Z \in [-1, 1]\), \(Y \in [0, 1]\).
pub const BOX_KIT_MIN: Vec3 = Vec3::new(-1.0, 0.0, -1.0);
/// See [`BOX_KIT_MIN`].
pub const BOX_KIT_MAX: Vec3 = Vec3::new(1.0, 1.0, 1.0);

/// Latch after remap: front-centroid, \(X,Y \in [-1, 1]\), \(Z \in [0, 1]\) inward.
///
/// Authored Blender: \(Y \in [0, 1]\) (depth from the front), \(X,Z \in [-1, 1]\).
pub const LATCH_KIT_MIN: Vec3 = Vec3::new(-1.0, -1.0, 0.0);
/// See [`LATCH_KIT_MIN`].
pub const LATCH_KIT_MAX: Vec3 = Vec3::new(1.0, 1.0, 1.0);

/// Authored chair-leg tube after remap: plan \(X,Z \in [-0.2, 0.2]\), \(Y \in [0, 1]\).
pub const LEG_KIT_MIN: Vec3 = Vec3::new(-0.2, 0.0, -0.2);
/// See [`LEG_KIT_MIN`].
pub const LEG_KIT_MAX: Vec3 = Vec3::new(0.2, 1.0, 0.2);

/// Box kit → unit slot: origin on the unit-cube floor, extents fill the cube.
pub const BOX_KIT_TO_UNIT: Placement = Placement {
	translation: Vec3::new(0.0, -0.5, 0.0),
	yaw: 0.0,
	pitch: 0.0,
	roll: 0.0,
	scale: Vec3::new(0.5, 1.0, 0.5),
};

/// Latch kit → unit slot: origin pinned on the \(−Z\) face, face toward the room.
///
/// Authored \(X,Y \in [-1, 1]\) are centered (`scale = 0.5`). Authored \(+Z\)
/// is depth from the front centroid; other kits treat \(+Z\) as the back, so
/// yaw \(\pi\) turns that axis toward the room. The origin stays on \(z=-0.5\).
/// Use [`latch_slab`] so floor-fraction \(y0..y1\) still lands correctly.
pub const LATCH_KIT_TO_UNIT: Placement = Placement {
	translation: Vec3::new(0.0, 0.0, -0.5),
	yaw: std::f32::consts::PI,
	pitch: 0.0,
	roll: 0.0,
	scale: Vec3::new(0.5, 0.5, 0.5),
};

/// Leg tube → unit slot (same floor origin).
pub const LEG_KIT_TO_UNIT: Placement = Placement {
	translation: Vec3::new(0.0, -0.5, 0.0),
	yaw: 0.0,
	pitch: 0.0,
	roll: 0.0,
	scale: Vec3::new(2.5, 1.0, 2.5),
};

/// Fridge / wardrobe hinge door: authored \(X \in [0,1]\), thin \(Y\), \(Z \in [0,1]\).
///
/// After remap the hinge sits at the origin, the panel goes \(+X\), and depth
/// is thin about \(Z=0\). Pin the hinge on the unit-cube floor / front
/// (\(y=-0.5\), \(z=-0.5\)); \(X[0,1]\) fills half the slot width.
pub const HINGE_KIT_TO_UNIT: Placement = Placement {
	translation: Vec3::new(0.0, -0.5, -0.5),
	yaw: 0.0,
	pitch: 0.0,
	roll: 0.0,
	scale: Vec3::new(0.5, 1.0, 1.0),
};

/// Range oven door: authored \(X \in [-1,1]\), thin \(Y\), \(Z \in [0,1]\) up.
///
/// After remap this is a standing front panel. Pin the bottom-front hinge on
/// the unit-cube front; keep authored depth (do not fatten \(Z\) to the slot).
pub const RANGE_DOOR_KIT_TO_UNIT: Placement = Placement {
	translation: Vec3::new(0.0, -0.5, -0.5),
	yaw: 0.0,
	pitch: 0.0,
	roll: 0.0,
	scale: Vec3::new(0.5, 1.0, 1.0),
};

/// Floor-origin slab in the unit slot cube: \(y\) from the floor (\(y=-0.5\))
/// as height fractions; \(x,z\) scale in \([0,1]\) of the slot plan.
///
/// Kit GLBs keep their authored floor origin; do not use a centered cuboid.
pub fn slab(xz_scale: f32, y0: f32, y1: f32) -> Placement {
	let y0 = y0.clamp(0.0, 1.0);
	let y1 = y1.clamp(y0 + 1e-4, 1.0);
	Placement {
		translation: Vec3::new(0.0, y0, 0.0),
		yaw: 0.0,
		pitch: 0.0,
		roll: 0.0,
		scale: Vec3::new(xz_scale.max(1e-4), (y1 - y0).max(1e-4), xz_scale.max(1e-4)),
	}
}

/// Slab with independent plan scales (counters).
pub fn slab_xz(scale_x: f32, scale_z: f32, y0: f32, y1: f32) -> Placement {
	let mut p = slab(1.0, y0, y1);
	p.scale.x = scale_x.max(1e-4);
	p.scale.z = scale_z.max(1e-4);
	p
}

/// Length `scale_x` (1 = flush ends). Depth `scale_z`. When `flush_pos_z`,
/// the \(+Z\) face sits on the unit-cube wall (kit facing / abutment).
pub fn run_slab(scale_x: f32, scale_z: f32, y0: f32, y1: f32, flush_pos_z: bool) -> Placement {
	let mut p = slab_xz(scale_x, scale_z, y0, y1);
	if flush_pos_z {
		p.translation.z = 0.5 - p.scale.z * 0.5;
	}
	p
}

/// Front-centroid latch in the unit slot: \(y0..y1\) are floor-origin height
/// fractions; `width` / `depth` are slot-plan fractions. The kit front stays
/// on the unit-cube \(−Z\) face.
pub fn latch_slab(width: f32, y0: f32, y1: f32, depth: f32) -> Placement {
	let y0 = y0.clamp(0.0, 1.0);
	let y1 = y1.clamp(y0 + 1e-4, 1.0);
	Placement {
		translation: Vec3::new(0.0, y0 + y1 - 1.0, 0.0),
		yaw: 0.0,
		pitch: 0.0,
		roll: 0.0,
		scale: Vec3::new(width.max(1e-4), (y1 - y0).max(1e-4), depth.max(1e-4)),
	}
}

/// Offset a placement in unit-slot coordinates (fractions of the parent cube).
pub fn shift(mut placement: Placement, delta: Vec3) -> Placement {
	placement.translation += delta;
	placement
}

/// Map a unit-slot slab onto an authored kit, then into `slot`.
///
/// Plan offsets on `unit` stay in unit-slot space. [`compose_child`] would
/// otherwise scale them by [`LEG_KIT_TO_UNIT`] (2.5) and throw chair legs
/// outside the seat.
pub fn place_kit(slot: Placement, kit_to_unit: Placement, unit: Placement) -> Placement {
	let plan_offset = Vec3::new(unit.translation.x, 0.0, unit.translation.z);
	let mut centered = unit;
	centered.translation.x = 0.0;
	centered.translation.z = 0.0;
	let mut local = kit_to_unit.compose_child(centered);
	local.translation += plan_offset;
	slot.compose_child(local)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn box_kit_to_unit_puts_the_floor_on_the_slot_floor() -> anyhow::Result<()> {
		let floor = BOX_KIT_TO_UNIT.compose_child(Placement::IDENTITY);
		if (floor.translation.y + 0.5).abs() > 1e-5 {
			return Err(anyhow::anyhow!("expected floor at y=-0.5, got {}", floor.translation.y));
		}
		if (floor.scale - Vec3::new(0.5, 1.0, 0.5)).length() > 1e-5 {
			return Err(anyhow::anyhow!("box kit scale should be (0.5, 1, 0.5)"));
		}
		Ok(())
	}

	#[test]
	fn plan_offsets_are_not_scaled_by_the_leg_kit() -> anyhow::Result<()> {
		let unit = shift(slab(0.12, 0.0, 0.42), Vec3::new(0.32, 0.0, 0.32));
		let placed = place_kit(Placement::IDENTITY, LEG_KIT_TO_UNIT, unit);
		if (placed.translation.x - 0.32).abs() > 1e-4 || (placed.translation.z - 0.32).abs() > 1e-4
		{
			return Err(anyhow::anyhow!(
				"leg plan offset should stay 0.32 in unit space, got ({}, {})",
				placed.translation.x,
				placed.translation.z
			));
		}
		Ok(())
	}

	#[test]
	fn floor_slab_keeps_the_kit_origin_on_the_band_floor() -> anyhow::Result<()> {
		let band = slab(0.92, 0.32, 0.92);
		if (band.translation.y - 0.32).abs() > 1e-5 {
			return Err(anyhow::anyhow!("slab origin should sit on y0"));
		}
		if (band.scale.y - 0.60).abs() > 1e-5 {
			return Err(anyhow::anyhow!("slab height should be y1-y0"));
		}
		Ok(())
	}

	#[test]
	fn latch_slab_pins_the_front_and_the_height_band() -> anyhow::Result<()> {
		let unit = latch_slab(0.22, 0.56, 0.74, 0.12);
		let placed = place_kit(Placement::IDENTITY, LATCH_KIT_TO_UNIT, unit);
		let front = placed.translation.z;
		if (front + 0.5).abs() > 1e-4 {
			return Err(anyhow::anyhow!("latch origin should stay pinned at z=-0.5, got {front}"));
		}
		if (placed.yaw - std::f32::consts::PI).abs() > 1e-4 {
			return Err(anyhow::anyhow!(
				"latch yaw should turn the face toward the room, got {}",
				placed.yaw
			));
		}
		let y_lo = placed.translation.y - placed.scale.y;
		let y_hi = placed.translation.y + placed.scale.y;
		if (y_lo + 0.5 - 0.56).abs() > 1e-4 || (y_hi + 0.5 - 0.74).abs() > 1e-4 {
			return Err(anyhow::anyhow!(
				"latch Y should span floor-frac 0.56..0.74, got {}..{}",
				y_lo + 0.5,
				y_hi + 0.5
			));
		}
		Ok(())
	}

	#[test]
	fn flush_run_puts_the_pos_z_face_on_the_wall() -> anyhow::Result<()> {
		let run = run_slab(1.0, 0.72, 0.0, 0.12, true);
		let face = run.translation.z + run.scale.z * 0.5;
		if (face - 0.5).abs() > 1e-5 {
			return Err(anyhow::anyhow!("flush +Z face should sit at 0.5, got {face}"));
		}
		if (run.scale.x - 1.0).abs() > 1e-5 {
			return Err(anyhow::anyhow!("run length should stay flush"));
		}
		Ok(())
	}
}
