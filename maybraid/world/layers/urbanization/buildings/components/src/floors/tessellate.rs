//! Private floor kit tessellation (not part of the public IR).

use crate::arc_kit::{decompose_arc_sweep, ArcKit};
use crate::floors::geometry::FloorGeometry;
use crate::floors::style::FloorStyle;
use crate::panels::{PanelGeometry, PanelKitCaps, Rectangle, RightTriangle};
use crate::placed::{Placed, Placement};
use scene_ref::MirrorAxis;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum FloorKit {
	Rectangle,
	RightTriangle { mirror: Option<MirrorAxis> },
	ArcFill(ArcKit),
	StructFill,
	CircleInscribedSquare,
}

impl FloorGeometry {
	pub(crate) fn placed_kits_for_style(
		&self,
		style: FloorStyle,
		parent: Placement,
	) -> Vec<Placed<FloorKit>> {
		let panel_caps = PanelKitCaps::from(style);
		self.kit_pieces(panel_caps)
			.into_iter()
			.map(|child| Placed {
				geom: child.geom,
				placement: parent.compose_child(child.placement),
			})
			.collect()
	}

	fn kit_pieces(&self, panel_caps: PanelKitCaps) -> Vec<Placed<FloorKit>> {
		match self {
			Self::Rectangle(_) => vec![Placed::at_origin(FloorKit::Rectangle)],
			Self::StructFill(_) => vec![Placed::at_origin(FloorKit::StructFill)],
			Self::CircleInscribedSquare(_) => {
				vec![Placed::at_origin(FloorKit::CircleInscribedSquare)]
			}
			Self::ArcFill(g) => decompose_arc_sweep(g.sweep_degrees)
				.into_iter()
				.map(|(kit, yaw)| Placed::new(FloorKit::ArcFill(kit), bevy_math::Vec3::ZERO, yaw))
				.collect(),
			Self::TessellatedTriangle(t) => {
				map_leaves(PanelGeometry::TessellatedTriangle(*t).flatten(panel_caps))
			}
		}
	}
}

fn map_leaves(pieces: Vec<Placed<PanelGeometry>>) -> Vec<Placed<FloorKit>> {
	pieces
		.into_iter()
		.filter_map(|p| {
			let kit = match p.geom {
				PanelGeometry::Rectangle(Rectangle) => FloorKit::Rectangle,
				PanelGeometry::RightTriangle(RightTriangle { mirror }) => {
					FloorKit::RightTriangle { mirror }
				}
				_ => return None,
			};
			Some(Placed { geom: kit, placement: p.placement })
		})
		.collect()
}
