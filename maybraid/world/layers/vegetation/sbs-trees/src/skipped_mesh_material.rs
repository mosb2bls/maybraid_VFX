//! Helpers for embedding [`MeshMaterial3d`] in CLI-driven configs without exposing handles as flags yet.
//!
//! When a command flattens **two** skipped material slots (stick + leaf), each wrapper needs a distinct
//! [`clap::Args`] group id — otherwise clap panics with “Argument group name must be unique”.

use bevy::prelude::*;

macro_rules! define_skipped_mesh_material {
	($name:ident, $group_id:literal) => {
		/// Wraps a [`MeshMaterial3d`] for [`clap::Args`] flattening (all fields skipped).
		#[derive(Clone, Debug, PartialEq, clap::Args)]
		#[group(id = $group_id)]
		pub struct $name<M: Material> {
			#[arg(skip)]
			pub mesh: MeshMaterial3d<M>,
		}

		impl<M: Material> Default for $name<M> {
			fn default() -> Self {
				Self { mesh: MeshMaterial3d::default() }
			}
		}

		impl<M: Material> From<MeshMaterial3d<M>> for $name<M> {
			fn from(mesh: MeshMaterial3d<M>) -> Self {
				Self { mesh }
			}
		}

		impl<M: Material> From<$name<M>> for MeshMaterial3d<M> {
			fn from(wrapped: $name<M>) -> Self {
				wrapped.mesh
			}
		}
	};
}

define_skipped_mesh_material!(SkippedStickMeshMaterial, "stick-mesh-material");
define_skipped_mesh_material!(SkippedLeafMeshMaterial, "leaf-mesh-material");
define_skipped_mesh_material!(SkippedInnerLeafMeshMaterial, "inner-leaf-mesh-material");
define_skipped_mesh_material!(SkippedOuterLeafMeshMaterial, "outer-leaf-mesh-material");

/// Alias for single-material commands (same clap group as [`SkippedStickMeshMaterial`]).
pub type SkippedMeshMaterial<M> = SkippedStickMeshMaterial<M>;
