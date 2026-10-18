//! [`BuildingComponents`] presentation for [`PanelComplex`].

use building_components::joints::JointNode;
use building_components::panels::PanelNode;
use building_components::{BuildingComponents, Layers};
use lod::scene::LodSceneLevel;

use super::types::PanelComplex;

impl BuildingComponents for PanelComplex {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for panel in self.triangle_panels() {
			out.extend(panel.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<JointNode> {
		Layers::from_free(self.joint_nodes())
	}
}
