//! Re-export clip identity from `character-motion`, plus concept-screen maps.
//!
//! `From<ConceptAnimation>` stays here so motion does not depend on recipes.

pub use character_motion::{
	apply_anim_mailbox, prepare_anim_mailbox, tick_anim_mailbox, AnimBone, AnimClip, AnimId,
	AnimMailbox, AnimProgress, AnimRef, AnimRefRoot, JabParams, JumpParams, TuckParams,
	TuckedFlipParams, TwoFootedTuckedFlipParams,
};

use crate::concepts::ConceptAnimation;

impl From<ConceptAnimation> for AnimId {
	fn from(value: ConceptAnimation) -> Self {
		AnimClip::from(value).id()
	}
}

impl From<ConceptAnimation> for AnimClip {
	fn from(value: ConceptAnimation) -> Self {
		match value {
			ConceptAnimation::Still => Self::still(),
			ConceptAnimation::Walk => Self::walk(),
			ConceptAnimation::Run => Self::run(),
			ConceptAnimation::Gallop => Self::gallop(),
			ConceptAnimation::Jump => Self::jump(),
			ConceptAnimation::Leap => Self::leap(),
			ConceptAnimation::Tuck => Self::tuck(),
			ConceptAnimation::TuckedFlip => Self::tucked_flip(),
			ConceptAnimation::TwoFootedTuckedFlip => Self::two_footed_tucked_flip(),
			ConceptAnimation::Soaring => Self::soaring(),
			ConceptAnimation::Flapping => Self::flapping(),
			ConceptAnimation::Jab => Self::jab(),
			ConceptAnimation::Squat => Self::squat(),
			ConceptAnimation::Prone => Self::prone(),
			ConceptAnimation::LateralUndulation => Self::lateral_undulation(),
			ConceptAnimation::DorsoventralUndulation => Self::dorsoventral_undulation(),
		}
	}
}

impl From<ConceptAnimation> for AnimRef {
	fn from(value: ConceptAnimation) -> Self {
		Self::new(AnimClip::from(value))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use character_motion::clip::{IDLE_CYCLE_SPEED, LEAP_CYCLE_SPEED};

	#[test]
	fn concept_animation_maps_to_anim_ref() {
		let walk = AnimRef::from(ConceptAnimation::Walk);
		assert_eq!(walk.clip.id(), AnimId::Walk);
		assert_eq!(walk.clip, AnimClip::walk());
		assert_eq!(walk.speed, AnimId::Walk.default_speed());
	}

	#[test]
	fn home_still_maps_to_still_clip() {
		let still = AnimRef::from(ConceptAnimation::Still);
		assert_eq!(still.clip.id(), AnimId::Still);
		assert_eq!(still.clip, AnimClip::still());
		assert_eq!(still.speed, IDLE_CYCLE_SPEED);
	}

	#[test]
	fn leap_is_a_distinct_clip() {
		let leap = AnimRef::from(ConceptAnimation::Leap);
		assert_eq!(leap.clip.id(), AnimId::Leap);
		assert_eq!(leap.clip, AnimClip::leap());
		assert_eq!(leap.speed, LEAP_CYCLE_SPEED);
		assert_ne!(AnimClip::leap().id(), AnimClip::jump().id());
	}
}
