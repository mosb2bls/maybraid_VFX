//! Triangle mesh of panel points with crease joints on shared edges.
//!
//! Authors insert [`PanelPoint`]s (position + thickness), then list
//! [`PanelTriangle`]s by id. Interior edges (two incident triangles) are
//! found in one pass via canonical undirected keys; a [`JointNode`] is
//! emitted when the dihedral kink meets [`PanelComplexJointPolicy`].
//!
//! Compact authoring forms (playground / scripts):
//! ```text
//! // triangles
//! 1=(0.5,0,0),2=(2.5,0,0),3=(0,0.3,3),4=(3,0,3) ... {1,2,4},{1,4,3}
//! // quads ({a0,a1,b0,b1}, diagonal a0–b1) — see [`PanelQuadMesh`] / QuadPanelComplex
//! 1=(0,0,0),2=(1,0,0),3=(0,1,0),4=(0,0,1) ... {1,2,3,4}
//! ```

mod adjacency;
mod mesh;
mod mutate;
mod parse;
mod present;
mod quad;
mod query;
mod types;

#[cfg(test)]
mod tests;

pub use adjacency::shared_edges;
pub use mesh::PanelMesh;
pub use quad::PanelQuadMesh;
pub use types::{
	PanelComplex, PanelComplexJointPolicy, PanelComplexValidation, PanelPoint, PanelPointId,
	PanelTriangle, SharedEdge, DEFAULT_PANEL_THICKNESS,
};

pub use parse::ParsePanelComplexError;
