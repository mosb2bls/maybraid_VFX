//! Shared “room against a host wall + sales-face door + enclosure panels” packer.
//!
//! Used by MiniMart / Parts offices and PublicRestroom toilet stalls. Callers keep
//! domain mins, reserves, door catalogs, and post-pack fill (aisles, sinks, …).

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};
use procedural_common::{
	aabb2_area, clamp_min_size2, intersects_aabb2, plan_to_aabb3, Aabb2dPack, PlanAxes,
	PlanOpeningFace,
};

use crate::constraints::FaceKind;
use crate::openings::{Opening, OpeningId};
use crate::paneling::{Rectangle, DEFAULT_PANEL_THICKNESS};
use crate::usage_areas::clearance::{approach_blocked, PlanHost};
use crate::usage_areas::enclosure_panels::{face_rectangle, face_span_rectangle};

/// Plan minimums for an enclosed room (long/short axes after packing).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnclosedRoomMins {
	pub long: f32,
	pub short: f32,
}

impl EnclosedRoomMins {
	pub fn square(min: f32) -> Self {
		Self { long: min, short: min }
	}

	pub fn ok(self, room: Aabb2d) -> bool {
		let w = room.max.x - room.min.x;
		let d = room.max.y - room.min.y;
		let (long, short) = if w >= d { (w, d) } else { (d, w) };
		long + 1e-3 >= self.long && short + 1e-3 >= self.short
	}

	fn min_area(self) -> f32 {
		self.long * self.short
	}
}

/// Knobs for [`EnclosedRoomParams::pack`] / [`EnclosedRoomParams::pack_filtered`].
#[derive(Debug, Clone, PartialEq)]
pub struct EnclosedRoomParams {
	pub mins: EnclosedRoomMins,
	pub contact: f32,
	pub seed_depth: f32,
	pub along_t: f32,
	pub area_target: f32,
	/// Area reserved for the rest of the bay when clamping the grow target.
	/// Caller should already fold domain floors (aisles, sinks, …) into this value.
	pub area_reserve: f32,
	/// Cap on [`Self::area_reserve`] as a fraction of host area.
	pub reserve_cap_frac: f32,
	/// Also `grow_into` after `grow_toward_area` (restroom fills harder).
	pub grow_into: bool,
	/// When set, clamp the packed room so each plan axis spans at most this
	/// fraction of the host (e.g. `0.5` → never more than half the bay on X or Z).
	pub max_axis_frac: Option<f32>,
	/// Pull the free/sales face back so a door clearance band fits in-host.
	/// Restroom usually leaves a reserved strip instead and sets this false.
	pub shrink_sales_for_door_clear: bool,
	pub door_width: f32,
	pub door_width_min: f32,
	pub door_width_max: f32,
	pub door_along_t: f32,
	pub door_height: f32,
	pub door_height_min: f32,
	pub door_height_max: f32,
	pub door_header_min: f32,
	/// Inward clearance depth for the sales-face door keep-out (usually [`PASSAGE_CLEARANCE`]).
	pub door_clearance: f32,
	pub door_id: OpeningId,
}

/// Packed enclosed room on the plan + enclosure panels / door.
#[derive(Debug, Clone, PartialEq)]
pub struct EnclosedRoom {
	pub room: Aabb2d,
	pub seed_face: PlanOpeningFace,
	pub walls: Vec<Rectangle>,
	pub door_clear: Aabb2d,
	pub door_id: OpeningId,
	pub door: Opening,
}

impl EnclosedRoomParams {
	/// Pack with no per-face seed extras and no extra accept filter.
	pub fn pack(
		&self,
		host3: &Aabb3d,
		host: Aabb2d,
		clearances: &[Aabb2d],
	) -> Option<EnclosedRoom> {
		self.pack_filtered(host3, host, clearances, |_| Some(Vec::new()), |_, _| true)
	}

	/// Like [`Self::pack`], but:
	/// - `face_seed_extra` may add hard excludes per candidate face (`None` skips the face)
	/// - `accept` may reject a grown room after mins / contact checks (e.g. restroom door-side zone)
	///
	/// Each candidate face is finalized (door-clear shrink, axis clamp, host snap, enclose)
	/// and **rejected** when the door keep-out intersects existing `clearances`, so a blocked
	/// sales face does not win over a free wall.
	pub fn pack_filtered(
		&self,
		host3: &Aabb3d,
		host: Aabb2d,
		clearances: &[Aabb2d],
		mut face_seed_extra: impl FnMut(PlanOpeningFace) -> Option<Vec<Aabb2d>>,
		accept: impl Fn(Aabb2d, PlanOpeningFace) -> bool,
	) -> Option<EnclosedRoom> {
		for (mut room, seed_face) in
			self.iter_seeded_rooms(host, clearances, &mut face_seed_extra, &accept)
		{
			if self.shrink_sales_for_door_clear {
				let Some(shrunk) = shrink_for_door_clearance(
					host,
					room,
					seed_face,
					self.door_clearance + 0.05,
					self.mins,
					self.contact,
				) else {
					continue;
				};
				room = shrunk;
			}
			if let Some(frac) = self.max_axis_frac {
				let Some(clamped) =
					clamp_room_axis_frac(host, room, seed_face, frac, self.mins, self.contact)
				else {
					continue;
				};
				room = clamped;
			}
			// Pull near-host lateral faces flush so we omit thin partition walls in the gap.
			room = snap_near_host_sides(host, room, seed_face, clearances, HOST_WALL_SNAP);
			let Some(enclosure) = self.enclose(host3, host, room, seed_face) else {
				continue;
			};
			// Door approach must stay free of beds / furniture / other keep-outs.
			if approach_blocked(enclosure.door_clear, clearances) {
				continue;
			}
			return Some(EnclosedRoom {
				room,
				seed_face,
				walls: enclosure.walls,
				door_clear: enclosure.door_clear,
				door_id: self.door_id.clone(),
				door: enclosure.door,
			});
		}
		None
	}

	fn iter_seeded_rooms(
		&self,
		host: Aabb2d,
		clearances: &[Aabb2d],
		face_seed_extra: &mut impl FnMut(PlanOpeningFace) -> Option<Vec<Aabb2d>>,
		accept: &impl Fn(Aabb2d, PlanOpeningFace) -> bool,
	) -> Vec<(Aabb2d, PlanOpeningFace)> {
		let mut candidates: Vec<PlanOpeningFace> = PlanHost::faces(host).into_iter().collect();
		candidates.sort_by(|a, b| {
			let blocked = |wall: PlanOpeningFace| {
				clearances.iter().any(|c| {
					wall.shared_border_len(*c) + 1e-3 >= self.contact.min(wall.along_len())
				}) as u8
			};
			blocked(*a).cmp(&blocked(*b)).then_with(|| {
				b.along_len().partial_cmp(&a.along_len()).unwrap_or(std::cmp::Ordering::Equal)
			})
		});

		let contact = self.contact;
		let depth = self.seed_depth.max(self.mins.short);
		let usable = aabb2_area(host);
		let reserve = self.area_reserve.min(usable * self.reserve_cap_frac);
		let target = self
			.area_target
			.max(self.mins.min_area())
			.min((usable - reserve).max(self.mins.min_area()));

		let mut out = Vec::new();
		for face in candidates {
			if face.along_len() + 1e-3 < contact {
				continue;
			}
			let Some(extra) = face_seed_extra(face) else {
				continue;
			};
			let mut seed_hard = clearances.to_vec();
			seed_hard.extend(extra);
			let Some(seed) = face
				.seed_from_free(host, &seed_hard, contact, depth, self.along_t)
				.or_else(|| face.seed_from_free(host, &seed_hard, contact, depth, 0.5))
			else {
				continue;
			};
			let mut hard = seed_hard;
			hard.push(face.outward_block(host));
			let mut grown = seed.grow_toward_area(host, &hard, target);
			if self.grow_into {
				grown = grown.grow_into(host, &hard);
			}
			let Some(room) = clamp_min_size2(grown, Vec2::splat(self.mins.short)) else {
				continue;
			};
			if !room.is_clear_of(&hard) {
				continue;
			}
			if !self.mins.ok(room) {
				continue;
			}
			if face.shared_border_len(room) + 1e-3 < contact {
				continue;
			}
			if !accept(room, face) {
				continue;
			}
			out.push((room, face));
		}
		out
	}

	fn enclose(
		&self,
		host3: &Aabb3d,
		host: Aabb2d,
		room2: Aabb2d,
		seed_face: PlanOpeningFace,
	) -> Option<EnclosureGeom> {
		let room3 = plan_to_aabb3(host3, room2, PlanAxes::XZ);
		let sales_face = sales_face_kind(seed_face);
		let divider_thru = if seed_face.thru_is_x {
			if seed_face.inward_positive {
				room2.max.x
			} else {
				room2.min.x
			}
		} else if seed_face.inward_positive {
			room2.max.y
		} else {
			room2.min.y
		};

		let door_face = PlanOpeningFace {
			thru_is_x: seed_face.thru_is_x,
			thru: divider_thru,
			along0: if seed_face.thru_is_x { room2.min.y } else { room2.min.x },
			along1: if seed_face.thru_is_x { room2.max.y } else { room2.max.x },
			inward_positive: seed_face.inward_positive,
		};
		let door_face = door_face.clip_to_host(host)?;
		let along_len = door_face.along_len();
		let max_door = (along_len - 0.4).max(along_len * 0.5);
		let door_w = self
			.door_width
			.clamp(self.door_width_min, self.door_width_max)
			.min(max_door)
			.min(along_len * 0.85);
		if door_w + 1e-3 < self.door_width_min.min(along_len * 0.45) || along_len + 1e-3 < door_w {
			return None;
		}

		let (door0, door1, door_clear) = {
			let mut placed = None;
			for &t in &[self.door_along_t, 0.5, 0.25, 0.75] {
				let Some((d0, d1)) = place_along(door_face.along0, door_face.along1, door_w, t)
				else {
					continue;
				};
				if let Some(clear) = door_face.band(host, door_w, self.door_clearance, t) {
					placed = Some((d0, d1, clear));
					break;
				}
			}
			placed?
		};

		let host_h = (host3.max.y - host3.min.y).max(1.0);
		let door_h = self
			.door_height
			.clamp(self.door_height_min, self.door_height_max)
			.min((host_h - self.door_header_min).max(self.door_height_min));
		let u0 = ((door0 - door_face.along0) / along_len).clamp(0.0, 1.0);
		let u1 = ((door1 - door_face.along0) / along_len).clamp(0.0, 1.0);

		let mut walls = Vec::new();
		for face in [FaceKind::Front, FaceKind::Back, FaceKind::Left, FaceKind::Right] {
			if face == sales_face || side_on_host(room2, host, face) {
				continue;
			}
			if let Some(r) = face_rectangle(&room3, face, DEFAULT_PANEL_THICKNESS) {
				walls.push(r);
			}
		}

		let mut door_panels = 0usize;
		if u0 > 0.02 {
			if let Some(r) =
				face_span_rectangle(&room3, sales_face, 0.0, u0, DEFAULT_PANEL_THICKNESS)
			{
				walls.push(r);
				door_panels += 1;
			}
		}
		if u1 < 0.98 {
			if let Some(r) =
				face_span_rectangle(&room3, sales_face, u1, 1.0, DEFAULT_PANEL_THICKNESS)
			{
				walls.push(r);
				door_panels += 1;
			}
		}
		if door_h + self.door_header_min <= host_h + 1e-3 {
			let omin = Vec3::from(room3.min);
			let omax = Vec3::from(room3.max);
			let header_aabb =
				Aabb3d::from_min_max(Vec3::new(omin.x, omin.y + door_h, omin.z), omax);
			if let Some(r) =
				face_span_rectangle(&header_aabb, sales_face, u0, u1, DEFAULT_PANEL_THICKNESS)
			{
				walls.push(r);
				door_panels += 1;
			}
		}
		if door_panels == 0 {
			return None;
		}

		let band = (DEFAULT_PANEL_THICKNESS * 0.5 + 0.08).max(0.12);
		let a0 = door0.min(door1);
		let a1 = door0.max(door1);
		let y0 = host3.min.y;
		let y1 = y0 + door_h.max(0.5);
		let door_bounds = if seed_face.thru_is_x {
			Aabb3d::from_min_max(
				Vec3::new(divider_thru - band, y0, a0),
				Vec3::new(divider_thru + band, y1, a1),
			)
		} else {
			Aabb3d::from_min_max(
				Vec3::new(a0, y0, divider_thru - band),
				Vec3::new(a1, y1, divider_thru + band),
			)
		};

		Some(EnclosureGeom { walls, door_clear, door: Opening::passage(door_bounds) })
	}
}

struct EnclosureGeom {
	walls: Vec<Rectangle>,
	door_clear: Aabb2d,
	door: Opening,
}

/// Gap below which a partition face should flush to the host (omit the wall).
const HOST_WALL_SNAP: f32 = 0.45;

/// Shrink `room` so each plan axis is ≤ `frac` of the host. Keeps the seed-wall
/// contact; trims the sales face and centers the along-wall span.
fn clamp_room_axis_frac(
	host: Aabb2d,
	mut room: Aabb2d,
	seed_face: PlanOpeningFace,
	frac: f32,
	mins: EnclosedRoomMins,
	contact: f32,
) -> Option<Aabb2d> {
	let frac = frac.clamp(0.05, 1.0);
	let max_x = (host.max.x - host.min.x) * frac;
	let max_y = (host.max.y - host.min.y) * frac;

	if seed_face.thru_is_x {
		let span = room.max.x - room.min.x;
		if span > max_x + 1e-3 {
			if seed_face.inward_positive {
				room.max.x = room.min.x + max_x;
			} else {
				room.min.x = room.max.x - max_x;
			}
		}
		let span = room.max.y - room.min.y;
		if span > max_y + 1e-3 {
			let mid = 0.5 * (room.min.y + room.max.y);
			room.min.y = (mid - 0.5 * max_y).max(host.min.y);
			room.max.y = (room.min.y + max_y).min(host.max.y);
			room.min.y = (room.max.y - max_y).max(host.min.y);
		}
	} else {
		let span = room.max.y - room.min.y;
		if span > max_y + 1e-3 {
			if seed_face.inward_positive {
				room.max.y = room.min.y + max_y;
			} else {
				room.min.y = room.max.y - max_y;
			}
		}
		let span = room.max.x - room.min.x;
		if span > max_x + 1e-3 {
			let mid = 0.5 * (room.min.x + room.max.x);
			room.min.x = (mid - 0.5 * max_x).max(host.min.x);
			room.max.x = (room.min.x + max_x).min(host.max.x);
			room.min.x = (room.max.x - max_x).max(host.min.x);
		}
	}

	if room.max.x - room.min.x < 1e-3 || room.max.y - room.min.y < 1e-3 {
		return None;
	}
	if !mins.ok(room) {
		return None;
	}
	if seed_face.shared_border_len(room) + 1e-3 < contact {
		return None;
	}
	Some(room)
}

/// Expand lateral (and seed) faces that sit within `snap` of the host when the
/// intervening strip is clear. Never snaps the sales face — that keeps door
/// clearance. Flushed faces then skip enclosure panels via [`side_on_host`].
fn snap_near_host_sides(
	host: Aabb2d,
	mut room: Aabb2d,
	seed_face: PlanOpeningFace,
	clearances: &[Aabb2d],
	snap: f32,
) -> Aabb2d {
	let sales = sales_face_kind(seed_face);

	// Seed wall: already meant to ride the host; flush if slightly inset.
	if seed_face.thru_is_x {
		if seed_face.inward_positive {
			try_snap_edge(&mut room, host, FaceKind::Left, clearances, snap);
		} else {
			try_snap_edge(&mut room, host, FaceKind::Right, clearances, snap);
		}
	} else if seed_face.inward_positive {
		try_snap_edge(&mut room, host, FaceKind::Front, clearances, snap);
	} else {
		try_snap_edge(&mut room, host, FaceKind::Back, clearances, snap);
	}

	// Lateral faces only (not sales).
	for face in [FaceKind::Front, FaceKind::Back, FaceKind::Left, FaceKind::Right] {
		if face == sales {
			continue;
		}
		try_snap_edge(&mut room, host, face, clearances, snap);
	}
	room
}

fn try_snap_edge(
	room: &mut Aabb2d,
	host: Aabb2d,
	face: FaceKind,
	clearances: &[Aabb2d],
	snap: f32,
) {
	let gap = match face {
		FaceKind::Front => room.min.y - host.min.y,
		FaceKind::Back => host.max.y - room.max.y,
		FaceKind::Left => room.min.x - host.min.x,
		FaceKind::Right => host.max.x - room.max.x,
		FaceKind::Top | FaceKind::Bottom => return,
	};
	if gap <= 1e-4 || gap > snap + 1e-4 {
		return;
	}
	let strip = match face {
		FaceKind::Front => Aabb2d {
			min: Vec2::new(room.min.x, host.min.y),
			max: Vec2::new(room.max.x, room.min.y),
		},
		FaceKind::Back => Aabb2d {
			min: Vec2::new(room.min.x, room.max.y),
			max: Vec2::new(room.max.x, host.max.y),
		},
		FaceKind::Left => Aabb2d {
			min: Vec2::new(host.min.x, room.min.y),
			max: Vec2::new(room.min.x, room.max.y),
		},
		FaceKind::Right => Aabb2d {
			min: Vec2::new(room.max.x, room.min.y),
			max: Vec2::new(host.max.x, room.max.y),
		},
		FaceKind::Top | FaceKind::Bottom => return,
	};
	if strip.max.x - strip.min.x < 1e-4 || strip.max.y - strip.min.y < 1e-4 {
		return;
	}
	if clearances.iter().any(|c| intersects_aabb2(strip, *c)) {
		return;
	}
	match face {
		FaceKind::Front => room.min.y = host.min.y,
		FaceKind::Back => room.max.y = host.max.y,
		FaceKind::Left => room.min.x = host.min.x,
		FaceKind::Right => room.max.x = host.max.x,
		FaceKind::Top | FaceKind::Bottom => {}
	}
}

fn shrink_for_door_clearance(
	host: Aabb2d,
	mut room: Aabb2d,
	seed_face: PlanOpeningFace,
	need: f32,
	mins: EnclosedRoomMins,
	contact: f32,
) -> Option<Aabb2d> {
	if seed_face.thru_is_x {
		if seed_face.inward_positive {
			room.max.x = room.max.x.min(host.max.x - need);
		} else {
			room.min.x = room.min.x.max(host.min.x + need);
		}
	} else if seed_face.inward_positive {
		room.max.y = room.max.y.min(host.max.y - need);
	} else {
		room.min.y = room.min.y.max(host.min.y + need);
	}
	if room.max.x - room.min.x < 1e-3 || room.max.y - room.min.y < 1e-3 {
		return None;
	}
	if !mins.ok(room) {
		return None;
	}
	if seed_face.shared_border_len(room) + 1e-3 < contact {
		return None;
	}
	Some(room)
}

fn sales_face_kind(seed_face: PlanOpeningFace) -> FaceKind {
	match (seed_face.thru_is_x, seed_face.inward_positive) {
		(true, true) => FaceKind::Right,
		(true, false) => FaceKind::Left,
		(false, true) => FaceKind::Back,
		(false, false) => FaceKind::Front,
	}
}

fn side_on_host(room2: Aabb2d, host: Aabb2d, face: FaceKind) -> bool {
	// After [`snap_near_host_sides`], near-host faces are flush; tolerate float noise.
	const EPS: f32 = 0.08;
	match face {
		FaceKind::Front => (room2.min.y - host.min.y).abs() < EPS,
		FaceKind::Back => (room2.max.y - host.max.y).abs() < EPS,
		FaceKind::Left => (room2.min.x - host.min.x).abs() < EPS,
		FaceKind::Right => (room2.max.x - host.max.x).abs() < EPS,
		FaceKind::Top | FaceKind::Bottom => true,
	}
}

fn place_along(a0: f32, a1: f32, len: f32, t: f32) -> Option<(f32, f32)> {
	let span = a1 - a0;
	if span + 1e-4 < len {
		return None;
	}
	let t = t.clamp(0.0, 1.0);
	let start = a0 + (span - len) * t;
	Some((start, start + len))
}
