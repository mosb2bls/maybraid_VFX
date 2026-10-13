//! Shared stamp output bundle.

mod strength;

pub use strength::{scale_additive, scale_near_one, StampStrength};

use crate::modulation::StampModulation;
use bevy_math::Vec2;

/// Reference short-edge for softmask densify / even grading helpers (not vertical relief).
pub const SOFTMASK_REFERENCE_SHORT: f32 = 400.0;

/// Non-geometric facts for later gameplay / hydrology hand-off ([RFC-105 §3.6](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-105-procedural-terrain)).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StampSemantics {
	/// Tags such as `bank`, `arroyo`, `spillway_ready`.
	pub tags: Vec<&'static str>,
	/// Shared drainage identity for hydrology-shaped stamp chains.
	pub drainage_id: Option<u32>,
	/// Shared complex identity for multi-part landforms.
	pub complex_id: Option<u32>,
}

impl StampSemantics {
	pub fn with_tag(mut self, tag: &'static str) -> Self {
		self.tags.push(tag);
		self
	}

	pub fn with_drainage_id(mut self, id: u32) -> Self {
		self.drainage_id = Some(id);
		self
	}

	pub fn with_complex_id(mut self, id: u32) -> Self {
		self.complex_id = Some(id);
		self
	}
}

/// Ordered modulations plus optional spine geometry from one stamp construction.
#[derive(Debug, Clone)]
pub struct StampSet {
	pub modulations: Vec<StampModulation>,
	pub spine: Vec<Vec2>,
	pub semantics: StampSemantics,
}

impl StampSet {
	pub fn empty() -> Self {
		Self { modulations: Vec::new(), spine: Vec::new(), semantics: StampSemantics::default() }
	}

	pub fn apply_elevation(&self, mut elevation: f32, x: f32, z: f32) -> f32 {
		for m in &self.modulations {
			elevation = m.modify_elevation(elevation, x, z);
		}
		elevation
	}

	/// Append another stamp's modulations/spine; prefer `other`'s ids when set.
	pub fn extend_with(&mut self, other: StampSet) {
		self.modulations.extend(other.modulations);
		if self.spine.is_empty() {
			self.spine = other.spine;
		} else {
			self.spine.extend(other.spine);
		}
		for tag in other.semantics.tags {
			if !self.semantics.tags.contains(&tag) {
				self.semantics.tags.push(tag);
			}
		}
		if other.semantics.drainage_id.is_some() {
			self.semantics.drainage_id = other.semantics.drainage_id;
		}
		if other.semantics.complex_id.is_some() {
			self.semantics.complex_id = other.semantics.complex_id;
		}
	}
}
