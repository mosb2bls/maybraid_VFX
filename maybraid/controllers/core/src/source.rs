//! One normalized hardware source. Today that is the local [`VirtualPad`].

use maybraid_input::VirtualPad;

/// Who produced a sample. Phase 1 has a single local pad resource.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum InputSource {
	#[default]
	LocalPad,
}

impl InputSource {
	pub const fn local_pad() -> Self {
		Self::LocalPad
	}

	/// The process-wide [`VirtualPad`] is the only hardware source today.
	pub fn matches_local_pad(self, _pad: &VirtualPad) -> bool {
		matches!(self, Self::LocalPad)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn local_pad_is_the_only_source() {
		let pad = VirtualPad::default();
		assert!(InputSource::local_pad().matches_local_pad(&pad));
		assert_eq!(InputSource::default(), InputSource::LocalPad);
	}
}
