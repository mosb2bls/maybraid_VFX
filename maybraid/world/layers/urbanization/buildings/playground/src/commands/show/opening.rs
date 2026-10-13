//! Shared `--opening` CLI parsing for shell previews.
//!
//! Formats:
//! - AABB: `id:label:minx,miny,minz:maxx,maxy,maxz`
//! - Arc ring locus: `id:label:t=0.5` (resolved with shell radius / height;
//!   optional `,ring=inner|outer` for circ-ring-floor)
//! - Ortho side: `id:label:side=south` (resolved with footprint / height)

use bevy::prelude::*;
use buildings::{
	ArcFloor, IFloor, Opening, OpeningId, OpeningLabel, Openings, RectFloor, RectFloorSide,
	Trazaloid, TrazaloidSide,
};

use super::transform::parse_vec3_csv;

/// One authored opening from the playground CLI.
#[derive(Clone, Debug, PartialEq)]
pub struct PreviewOpening {
	pub id: String,
	pub label: OpeningLabel,
	pub min: Vec3,
	pub max: Vec3,
}

impl PreviewOpening {
	pub fn bounds(&self) -> bevy_math::bounding::Aabb3d {
		bevy_math::bounding::Aabb3d::from_min_max(self.min.min(self.max), self.min.max(self.max))
	}

	pub fn into_pair(self) -> (OpeningId, Opening) {
		let bounds = self.bounds();
		(OpeningId::new(self.id), Opening::new(bounds, self.label))
	}
}

/// Parse `--opening` values into a plan [`Openings`] table.
pub fn openings_from_preview(openings: &[PreviewOpening]) -> Openings {
	let mut out = Openings::new();
	for opening in openings {
		let (id, o) = opening.clone().into_pair();
		out.insert(id, o);
	}
	out
}

/// Parse one `--opening` string.
///
/// - `id:label:x0,y0,z0:x1,y1,z1`
/// - `id:label:t=0.25` (arc-ring shorthand; requires arc context)
/// - `id:label:side=south` (ortho-side shorthand; requires ortho context)
pub fn parse_opening_arg(s: &str) -> Result<OpeningArg, String> {
	let s = s.trim();
	let parts: Vec<_> = s.split(':').collect();
	if parts.len() < 3 {
		return Err(format!(
			"expected id:label:min:max, id:label:t=…, or id:label:side=…, got {s:?}"
		));
	}
	let id = parts[0].trim();
	if id.is_empty() {
		return Err("opening id must be non-empty".into());
	}
	let label = parse_opening_label(parts[1].trim())?;
	let rest = parts[2..].join(":");
	let rest = rest.trim();
	if let Some(t_str) = rest.strip_prefix("t=") {
		let (t_part, ring) = split_t_and_ring(t_str.trim())?;
		let t: f32 = t_part.parse().map_err(|e| format!("t=: {e}"))?;
		return Ok(OpeningArg::ArcT { id: id.to_string(), label, t, ring });
	}
	if let Some(side_str) = rest.strip_prefix("side=") {
		let side = parse_ortho_side(side_str.trim())?;
		return Ok(OpeningArg::OrthoSide { id: id.to_string(), label, side });
	}
	// min:max where each is x,y,z — rest may be "x,y,z:x,y,z"
	let aabb_parts: Vec<_> = rest.splitn(2, ':').collect();
	if aabb_parts.len() != 2 {
		return Err(format!("expected minx,miny,minz:maxx,maxy,maxz after label, got {rest:?}"));
	}
	let min = parse_vec3_csv(aabb_parts[0].trim())?;
	let max = parse_vec3_csv(aabb_parts[1].trim())?;
	Ok(OpeningArg::Aabb { id: id.to_string(), label, min, max })
}

/// Preferred ring when resolving `t=` openings on a circular ring shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CircRingPreference {
	Outer,
	Inner,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OpeningArg {
	Aabb { id: String, label: OpeningLabel, min: Vec3, max: Vec3 },
	ArcT { id: String, label: OpeningLabel, t: f32, ring: Option<CircRingPreference> },
	OrthoSide { id: String, label: OpeningLabel, side: RectFloorSide },
}

fn split_t_and_ring(s: &str) -> Result<(&str, Option<CircRingPreference>), String> {
	if let Some((t_part, ring_part)) = s.split_once(',') {
		let ring_part = ring_part.trim();
		let ring = if let Some(r) = ring_part.strip_prefix("ring=") {
			match r.trim().to_ascii_lowercase().as_str() {
				"outer" | "o" => CircRingPreference::Outer,
				"inner" | "i" => CircRingPreference::Inner,
				other => {
					return Err(format!("unknown ring {other:?}; expected outer|inner"));
				}
			}
		} else {
			return Err(format!("expected t=…,ring=outer|inner after t=, got {s:?}"));
		};
		Ok((t_part.trim(), Some(ring)))
	} else {
		Ok((s, None))
	}
}

impl OpeningArg {
	/// Ring preference for `t=` openings (`None` ⇒ outer / single-radius shells).
	pub fn arc_ring_preference(&self) -> Option<CircRingPreference> {
		match self {
			Self::ArcT { ring, .. } => *ring,
			_ => None,
		}
	}

	pub fn resolve_aabb(self, arc: Option<ArcOpeningContext>) -> Result<PreviewOpening, String> {
		self.resolve(arc, None)
	}

	pub fn resolve(
		self,
		arc: Option<ArcOpeningContext>,
		ortho: Option<OrthoOpeningContext>,
	) -> Result<PreviewOpening, String> {
		match self {
			Self::Aabb { id, label, min, max } => Ok(PreviewOpening { id, label, min, max }),
			Self::ArcT { id, label, t, ring: _ } => {
				let ctx = arc.ok_or_else(|| {
					"opening t=… is only valid for arc-floor / arc-tower / circ-ring-floor previews"
						.to_string()
				})?;
				let (_id, opening) = ArcFloor::plan_opening_at_t(
					id.clone(),
					label.clone(),
					ctx.center_xz,
					ctx.radius,
					ctx.storey_height,
					t,
				);
				let min = Vec3::from(opening.bounds.min);
				let max = Vec3::from(opening.bounds.max);
				Ok(PreviewOpening { id, label, min, max })
			}
			Self::OrthoSide { id, label, side } => {
				let ctx = ortho.ok_or_else(|| {
					"opening side=… is only valid for rect-floor / rounded-rect-floor / i-floor / rect-ring-floor previews"
						.to_string()
				})?;
				let opening = match label {
					OpeningLabel::Aperture => RectFloor::side_aperture_opening(
						side,
						ctx.center_xz,
						ctx.footprint,
						ctx.door_width,
						ctx.door_height * 0.6,
						ctx.storey_height * 0.3,
					),
					_ => RectFloor::side_passage_opening(
						side,
						ctx.center_xz,
						ctx.footprint,
						ctx.door_width,
						ctx.door_height,
					),
				};
				let mut opening = opening;
				opening.label = label.clone();
				let min = Vec3::from(opening.bounds.min);
				let max = Vec3::from(opening.bounds.max);
				Ok(PreviewOpening { id, label, min, max })
			}
		}
	}
}

#[derive(Clone, Copy, Debug)]
pub struct ArcOpeningContext {
	pub center_xz: Vec3,
	pub radius: f32,
	pub storey_height: f32,
}

/// Context for `side=north|east|south|west` opening shorthand.
#[derive(Clone, Copy, Debug)]
pub struct OrthoOpeningContext {
	pub center_xz: Vec3,
	pub footprint: Vec2,
	pub storey_height: f32,
	pub door_width: f32,
	pub door_height: f32,
}

pub fn parse_ortho_side(s: &str) -> Result<RectFloorSide, String> {
	match s.trim().to_ascii_lowercase().as_str() {
		"north" | "n" => Ok(RectFloorSide::North),
		"east" | "e" => Ok(RectFloorSide::East),
		"south" | "s" => Ok(RectFloorSide::South),
		"west" | "w" => Ok(RectFloorSide::West),
		other => Err(format!("unknown side {other:?}; expected north|east|south|west")),
	}
}

pub fn parse_opening_label(s: &str) -> Result<OpeningLabel, String> {
	let s = s.trim().to_ascii_lowercase();
	match s.as_str() {
		"boundary" => Ok(OpeningLabel::Boundary),
		"exclusion" => Ok(OpeningLabel::Exclusion),
		"passage" | "door" => Ok(OpeningLabel::Passage),
		"aperture" | "window" => Ok(OpeningLabel::Aperture),
		"shaft" => Ok(OpeningLabel::Shaft),
		other if other.starts_with("custom=") => {
			Ok(OpeningLabel::Custom(other["custom=".len()..].to_string()))
		}
		other => Err(format!(
			"unknown opening label {other:?}; expected boundary|exclusion|passage|aperture|shaft|custom=…"
		)),
	}
}

/// Build trazaloid plan openings from CLI args, falling back to cardinal door flags.
///
/// Door width / height are absolute meters used only for the convenience `--door-*`
/// flags; the shell itself sizes doors from each winning passage AABB.
pub fn trazaloid_openings(
	args: &[OpeningArg],
	footprint: Vec2,
	door_width: f32,
	door_height: f32,
	door_north: bool,
	door_east: bool,
	door_south: bool,
	door_west: bool,
) -> Result<Vec<PreviewOpening>, String> {
	if !args.is_empty() {
		return args.iter().cloned().map(|a| a.resolve_aabb(None)).collect();
	}
	let mut out = Vec::new();
	for (enabled, side, id) in [
		(door_north, TrazaloidSide::North, "north"),
		(door_east, TrazaloidSide::East, "east"),
		(door_south, TrazaloidSide::South, "south"),
		(door_west, TrazaloidSide::West, "west"),
	] {
		if !enabled {
			continue;
		}
		let opening = Trazaloid::side_passage_opening(side, footprint, door_width, door_height);
		out.push(PreviewOpening {
			id: id.to_string(),
			label: OpeningLabel::Passage,
			min: Vec3::from(opening.bounds.min),
			max: Vec3::from(opening.bounds.max),
		});
	}
	Ok(out)
}

/// Build orthonormal-shell openings from CLI args / cardinal door flags.
///
/// Supports AABB, `side=…`, and `--door-*` convenience flags. Openings fit the
/// authored AABB position on the hit face (not centered).
pub fn ortho_openings(
	args: &[OpeningArg],
	ctx: OrthoOpeningContext,
	door_north: bool,
	door_east: bool,
	door_south: bool,
	door_west: bool,
) -> Result<Vec<PreviewOpening>, String> {
	if !args.is_empty() {
		return args.iter().cloned().map(|a| a.resolve(None, Some(ctx))).collect();
	}
	let mut out = Vec::new();
	for (enabled, side, id) in [
		(door_north, RectFloorSide::North, "north"),
		(door_east, RectFloorSide::East, "east"),
		(door_south, RectFloorSide::South, "south"),
		(door_west, RectFloorSide::West, "west"),
	] {
		if !enabled {
			continue;
		}
		let opening = RectFloor::side_passage_opening(
			side,
			ctx.center_xz,
			ctx.footprint,
			ctx.door_width,
			ctx.door_height,
		);
		out.push(PreviewOpening {
			id: id.to_string(),
			label: OpeningLabel::Passage,
			min: Vec3::from(opening.bounds.min),
			max: Vec3::from(opening.bounds.max),
		});
	}
	Ok(out)
}

/// I-floor openings: AABB / `side=…` / `--door-*` placed on the nearest outer edge.
pub fn i_floor_openings(
	args: &[OpeningArg],
	shell: &IFloor,
	ctx: OrthoOpeningContext,
	door_north: bool,
	door_east: bool,
	door_south: bool,
	door_west: bool,
) -> Result<Vec<PreviewOpening>, String> {
	if !args.is_empty() {
		let mut out = Vec::new();
		for arg in args.iter().cloned() {
			match arg {
				OpeningArg::OrthoSide { id, label, side } => {
					let edge = nearest_edge_for_side(shell, side)
						.ok_or_else(|| format!("i-floor has no edge near side={side:?}"))?;
					let opening = match label {
						OpeningLabel::Aperture => IFloor::edge_aperture_opening(
							edge,
							ctx.door_width,
							ctx.door_height * 0.6,
							ctx.storey_height * 0.3,
						),
						_ => IFloor::edge_passage_opening(edge, ctx.door_width, ctx.door_height),
					};
					let mut opening = opening;
					opening.label = label.clone();
					out.push(PreviewOpening {
						id,
						label,
						min: Vec3::from(opening.bounds.min),
						max: Vec3::from(opening.bounds.max),
					});
				}
				other => out.push(other.resolve(None, Some(ctx))?),
			}
		}
		return Ok(out);
	}
	let mut out = Vec::new();
	for (enabled, side, id) in [
		(door_north, RectFloorSide::North, "north"),
		(door_east, RectFloorSide::East, "east"),
		(door_south, RectFloorSide::South, "south"),
		(door_west, RectFloorSide::West, "west"),
	] {
		if !enabled {
			continue;
		}
		let Some(edge) = nearest_edge_for_side(shell, side) else {
			continue;
		};
		let opening = IFloor::edge_passage_opening(edge, ctx.door_width, ctx.door_height);
		out.push(PreviewOpening {
			id: id.to_string(),
			label: OpeningLabel::Passage,
			min: Vec3::from(opening.bounds.min),
			max: Vec3::from(opening.bounds.max),
		});
	}
	Ok(out)
}

fn nearest_edge_for_side(
	shell: &IFloor,
	side: RectFloorSide,
) -> Option<buildings::shells::ortho::WallEdge> {
	use buildings::shells::ortho::WallEdge;
	let edges = shell.edges();
	if edges.is_empty() {
		return None;
	}
	let target = match side {
		RectFloorSide::North => {
			let z = edges.iter().map(|e| e.mid().z).fold(f32::NEG_INFINITY, f32::max);
			Vec3::new(0.0, shell.params().center_xz.y + shell.params().storey_height * 0.5, z)
		}
		RectFloorSide::South => {
			let z = edges.iter().map(|e| e.mid().z).fold(f32::INFINITY, f32::min);
			Vec3::new(0.0, shell.params().center_xz.y + shell.params().storey_height * 0.5, z)
		}
		RectFloorSide::East => {
			let x = edges.iter().map(|e| e.mid().x).fold(f32::NEG_INFINITY, f32::max);
			Vec3::new(x, shell.params().center_xz.y + shell.params().storey_height * 0.5, 0.0)
		}
		RectFloorSide::West => {
			let x = edges.iter().map(|e| e.mid().x).fold(f32::INFINITY, f32::min);
			Vec3::new(x, shell.params().center_xz.y + shell.params().storey_height * 0.5, 0.0)
		}
	};
	edges.iter().copied().min_by(|a: &WallEdge, b: &WallEdge| {
		a.mid()
			.distance_squared(target)
			.partial_cmp(&b.mid().distance_squared(target))
			.unwrap_or(std::cmp::Ordering::Equal)
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parse_aabb_opening() -> Result<(), String> {
		let arg = parse_opening_arg("south:passage:-0.6,0,-3.2:0.6,2.1,-2.8")?;
		let preview = arg.resolve_aabb(None)?;
		assert_eq!(preview.id, "south");
		assert_eq!(preview.label, OpeningLabel::Passage);
		assert!((preview.min.x - -0.6).abs() < 1e-5);
		assert!((preview.max.y - 2.1).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn parse_arc_t_opening() -> Result<(), String> {
		let arg = parse_opening_arg("door:passage:t=0.0")?;
		let preview = arg.resolve_aabb(Some(ArcOpeningContext {
			center_xz: Vec3::ZERO,
			radius: 4.0,
			storey_height: 3.0,
		}))?;
		assert_eq!(preview.id, "door");
		assert!(preview.max.x > 3.0, "t=0 should resolve to +X");
		Ok(())
	}

	#[test]
	fn parse_ortho_side_opening() -> Result<(), String> {
		let arg = parse_opening_arg("south:passage:side=south")?;
		let preview = arg.resolve(
			None,
			Some(OrthoOpeningContext {
				center_xz: Vec3::ZERO,
				footprint: Vec2::new(8.0, 6.0),
				storey_height: 3.0,
				door_width: 1.2,
				door_height: 2.1,
			}),
		)?;
		assert_eq!(preview.id, "south");
		assert_eq!(preview.label, OpeningLabel::Passage);
		assert!(preview.min.z < -2.5, "south face z={}", preview.min.z);
		Ok(())
	}
}
