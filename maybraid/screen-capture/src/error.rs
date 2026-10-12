//! Completion, failure, and unsupported-material reporting.

use crate::key::CaptureCacheKey;
use bevy::prelude::{Handle, Image, Message};

/// Why a capture could not finish.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CaptureError {
	#[error("unsupported custom material: {reason}")]
	UnsupportedCustomMaterial { reason: String },
	#[error("capture scene did not become ready")]
	SceneNotReady,
	#[error("gpu readback failed")]
	Readback,
	#[error("invalid capture request: {reason}")]
	InvalidRequest { reason: String },
}

/// Where the returned image came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureSource {
	Cache,
	Rendered,
}

/// A capture finished and the image handle is usable.
#[derive(Clone, Debug, Message)]
pub struct CaptureCompleted {
	pub key: CaptureCacheKey,
	pub image: Handle<Image>,
	pub source: CaptureSource,
}

/// A capture could not produce an image.
#[derive(Clone, Debug, Message)]
pub struct CaptureFailed {
	pub key: CaptureCacheKey,
	pub error: CaptureError,
}

/// Non-fatal capture issue (shaded custom materials, failed disk write, …).
#[derive(Clone, Debug, Message)]
pub struct CaptureWarning {
	pub key: CaptureCacheKey,
	pub warning: CaptureWarningKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaptureWarningKind {
	UnsupportedCustomMaterial { reason: String },
	CacheWriteFailed { path: String, reason: String },
}

/// Ask the plugin to capture a scene.
#[derive(Clone, Debug, Message)]
pub struct RequestCapture(pub crate::request::CaptureRequest);
