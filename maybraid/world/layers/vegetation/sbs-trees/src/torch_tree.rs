//! Shared Penmarch / Kamakura stick and canopy helpers (also stick thinning for Rory).
//!
//! Both torches use [`StorybookTreeChain`](sbs_geometry::StorybookTreeChain) with the
//! same selective upper/outer cheap-ball foliage policy; this module holds that emission
//! logic once. Structural banding is [`vegetation_components::StructuralLod`] on each tree.

mod canopy;
mod stick;

pub(crate) use canopy::{
	foliage_nodes_banded, foliage_nodes_low, foliage_nodes_medium, HIGH_FOLIAGE_BANDS,
};
pub(crate) use stick::{
	stick_nodes_banded, stick_nodes_high, stick_nodes_low, stick_nodes_medium, HIGH_STICK_BANDS,
};
