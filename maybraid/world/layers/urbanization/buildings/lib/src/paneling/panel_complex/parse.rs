//! Compact string parsing for panel meshes.
//!
//! Triangle form:
//! ```text
//! 1=(0.5,0,0),2=(2.5,0,0),3=(0,0.3,3),4=(3,0,3) ... {1,2,4},{1,4,3}
//! ```
//!
//! Optional thickness as a 4th tuple component: `1=(0,0,0,0.25)`.

use std::str::FromStr;

use bevy_math::Vec3;
use building_components::panels::PanelStyle;

use super::mesh::PanelMesh;
use super::types::{PanelComplex, PanelPoint, PanelPointId};

/// Parse failure for the compact panel-complex syntax.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsePanelComplexError(pub String);

impl std::fmt::Display for ParsePanelComplexError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl std::error::Error for ParsePanelComplexError {}

impl FromStr for PanelComplex {
	type Err = ParsePanelComplexError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let mesh: PanelMesh = s.parse()?;
		Ok(PanelComplex::from_mesh(PanelStyle::RoughStonework, mesh))
	}
}

pub(super) fn err(msg: impl Into<String>) -> ParsePanelComplexError {
	ParsePanelComplexError(msg.into())
}

pub(super) fn split_mesh_src(s: &str) -> Result<(&str, &str), ParsePanelComplexError> {
	let s = s.trim();
	if s.is_empty() {
		return Err(err("empty panel-complex string"));
	}
	let (points_src, faces_src) = s
		.split_once("...")
		.ok_or_else(|| err("expected `points ... faces` with a `...` separator"))?;
	let points_src = points_src.trim();
	let faces_src = faces_src.trim();
	if points_src.is_empty() {
		return Err(err("missing point list before `...`"));
	}
	if faces_src.is_empty() {
		return Err(err("missing face list after `...`"));
	}
	Ok((points_src, faces_src))
}

pub(super) fn parse_points(
	src: &str,
) -> Result<Vec<(PanelPointId, PanelPoint)>, ParsePanelComplexError> {
	let mut out = Vec::new();
	let mut rest = src.trim();
	while !rest.is_empty() {
		rest = rest.trim_start();
		let eq = rest
			.find('=')
			.ok_or_else(|| err(format!("expected `id=(x,y,z)` near `{rest}`")))?;
		let id_src = rest[..eq].trim();
		let id: u32 = id_src.parse().map_err(|_| err(format!("invalid point id `{id_src}`")))?;
		rest = rest[eq + 1..].trim_start();
		if !rest.starts_with('(') {
			return Err(err(format!("expected `(` after id {id}")));
		}
		let close = rest.find(')').ok_or_else(|| err(format!("unclosed `(` for point {id}")))?;
		let inner = &rest[1..close];
		let point = parse_point_tuple(inner, id)?;
		out.push((PanelPointId(id), point));
		rest = rest[close + 1..].trim_start();
		if rest.starts_with(',') {
			rest = rest[1..].trim_start();
		} else if !rest.is_empty() {
			return Err(err(format!("expected `,` between points near `{rest}`")));
		}
	}
	Ok(out)
}

fn parse_point_tuple(inner: &str, id: u32) -> Result<PanelPoint, ParsePanelComplexError> {
	let parts: Vec<_> = inner.split(',').map(str::trim).filter(|p| !p.is_empty()).collect();
	match parts.as_slice() {
		[x, y, z] => {
			let position = Vec3::new(parse_f32(x, id)?, parse_f32(y, id)?, parse_f32(z, id)?);
			Ok(PanelPoint::at(position))
		}
		[x, y, z, t] => {
			let position = Vec3::new(parse_f32(x, id)?, parse_f32(y, id)?, parse_f32(z, id)?);
			Ok(PanelPoint::new(position, parse_f32(t, id)?))
		}
		_ => Err(err(format!("point {id}: expected (x,y,z) or (x,y,z,thickness), got ({inner})"))),
	}
}

fn parse_f32(s: &str, id: u32) -> Result<f32, ParsePanelComplexError> {
	s.parse::<f32>().map_err(|_| err(format!("point {id}: invalid number `{s}`")))
}

/// Parse `{id,…}` faces requiring exactly `arity` ids each.
pub(super) fn parse_faces(
	src: &str,
	arity: usize,
) -> Result<Vec<Vec<PanelPointId>>, ParsePanelComplexError> {
	let mut out = Vec::new();
	let mut rest = src.trim();
	while !rest.is_empty() {
		rest = rest.trim_start();
		if !rest.starts_with('{') {
			return Err(err(format!("expected `{{…}}` face near `{rest}`")));
		}
		let close = rest.find('}').ok_or_else(|| err(format!("unclosed `{{` near `{rest}`")))?;
		let inner = &rest[1..close];
		let parts: Vec<_> = inner.split(',').map(str::trim).filter(|p| !p.is_empty()).collect();
		if parts.len() != arity {
			return Err(err(format!("face expects {arity} ids, got {{{inner}}}")));
		}
		let mut ids = Vec::with_capacity(arity);
		for p in parts {
			ids.push(parse_face_id(p)?);
		}
		out.push(ids);
		rest = rest[close + 1..].trim_start();
		if rest.starts_with(',') {
			rest = rest[1..].trim_start();
		} else if !rest.is_empty() {
			return Err(err(format!("expected `,` between faces near `{rest}`")));
		}
	}
	Ok(out)
}

fn parse_face_id(s: &str) -> Result<PanelPointId, ParsePanelComplexError> {
	let id: u32 = s.parse().map_err(|_| err(format!("invalid face point id `{s}`")))?;
	Ok(PanelPointId(id))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_mild_trapezoid() {
		let c: PanelComplex = "1=(0.5,0,0),2=(2.5,0,0),3=(0,0.3,3),4=(3,0,3) ... {1,2,4},{1,4,3}"
			.parse()
			.expect("parse");
		assert_eq!(c.points().count(), 4);
		assert_eq!(c.triangles().len(), 2);
		assert_eq!(c.shared_edges().len(), 1);
		assert!(c.point(PanelPointId(1)).is_some());
		assert!(c.point(PanelPointId(0)).is_none());
	}

	#[test]
	fn parses_thickness_fourth_component() {
		let c: PanelComplex =
			"1=(0,0,0,0.2),2=(1,0,0,0.6),3=(0,0,1) ... {1,2,3}".parse().expect("parse");
		assert!((c.point(PanelPointId(1)).unwrap().thickness - 0.2).abs() < 1e-5);
		assert!((c.point(PanelPointId(2)).unwrap().thickness - 0.6).abs() < 1e-5);
	}
}
