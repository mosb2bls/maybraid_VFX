//! Shared panel geometry IR and kit capabilities.

mod kit_caps;
mod rectangle;
mod right_triangle;

pub use kit_caps::PanelKitCaps;
pub use rectangle::{fitted_tile_count, Rectangle, DEFAULT_TILE_WIDTH};
pub use right_triangle::RightTriangle;

use scene_ref::MirrorAxis;

use crate::placed::{Placed, Placement};

/// Shared panel geometry IR. All variants can [`PanelGeometry::decompose`].
///
/// Composition:
/// - tessellated triangles ← [`RightTriangle`] / [`crate::panels::TessellatedTriangle`]
/// - tessellated rectangles ← [`Rectangle`] (+ triangles when `!has_rectangle`)
#[derive(Debug, Clone, PartialEq)]
pub enum PanelGeometry {
	RightTriangle(RightTriangle),
	Rectangle(Rectangle),
	TessellatedTriangle(crate::panels::TessellatedTriangle),
}

impl Default for PanelGeometry {
	fn default() -> Self {
		Self::rectangle()
	}
}

impl PanelGeometry {
	pub fn rectangle() -> Self {
		Self::Rectangle(Rectangle)
	}

	pub fn right_triangle(mirror: Option<MirrorAxis>) -> Self {
		Self::RightTriangle(RightTriangle { mirror })
	}

	pub fn tessellated_triangle(t: crate::panels::TessellatedTriangle) -> Self {
		Self::TessellatedTriangle(t)
	}

	/// One-level decompose toward simpler variants.
	pub fn decompose(&self, caps: PanelKitCaps) -> Vec<Placed<PanelGeometry>> {
		match self {
			Self::RightTriangle(t) => {
				vec![Placed::at_origin(Self::RightTriangle(*t))]
			}
			Self::Rectangle(r) => {
				if caps.has_rectangle {
					vec![Placed::at_origin(Self::Rectangle(*r))]
				} else {
					// Unit square \(X \in [0,1]\), \(Z \in [-1,0]\) as dual kits
					// (identity + complement at lower-right with yaw π).
					use bevy_math::Vec3;
					use std::f32::consts::PI;
					vec![
						Placed::with_placement(
							Self::RightTriangle(RightTriangle { mirror: None }),
							Placement::new(Vec3::ZERO, 0.0),
						),
						Placed::with_placement(
							Self::RightTriangle(RightTriangle { mirror: None }),
							Placement::new(Vec3::new(1.0, 0.0, -1.0), PI),
						),
					]
				}
			}
			Self::TessellatedTriangle(t) => t.decompose(),
		}
	}

	/// Flatten composites to leaf atoms ([`Rectangle`], [`RightTriangle`]).
	pub fn flatten(&self, caps: PanelKitCaps) -> Vec<Placed<PanelGeometry>> {
		flatten_placed(Placed::at_origin(self.clone()), caps)
	}

	pub fn is_leaf_atom(&self) -> bool {
		matches!(self, Self::Rectangle(_) | Self::RightTriangle(_))
	}
}

fn flatten_placed(placed: Placed<PanelGeometry>, caps: PanelKitCaps) -> Vec<Placed<PanelGeometry>> {
	if placed.geom.is_leaf_atom() {
		// Rectangle may still expand to dual triangles under kit caps.
		if matches!(placed.geom, PanelGeometry::Rectangle(_)) && !caps.has_rectangle {
			return placed
				.geom
				.decompose(caps)
				.into_iter()
				.map(|child| Placed {
					geom: child.geom,
					placement: placed.placement.compose_child(child.placement),
				})
				.collect();
		}
		return vec![placed];
	}
	placed
		.geom
		.decompose(caps)
		.into_iter()
		.flat_map(|child| {
			flatten_placed(
				Placed {
					geom: child.geom,
					placement: placed.placement.compose_child(child.placement),
				},
				caps,
			)
		})
		.collect()
}
