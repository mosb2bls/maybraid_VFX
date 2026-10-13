//! Layered bind-pose composition — re-exported from [`rigs`].
//!
//! Absolute clip poses ([`crate::RigPose`]) stay here; proportion layers live
//! in the shared rig crate.

pub use ::rigs::{BoneRotation, BoneScale, BoneTranslation, ResolvedRigPose, RigPoseLayer};
