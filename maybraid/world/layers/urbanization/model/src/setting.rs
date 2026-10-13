//! Type-erased urban setting pin. Presentation spawns it; other layers read it.

use bevy::prelude::Component;
use lod::gen::Id;

/// Type-erased anchor for one presented urban setting.
///
/// The world layer turns this into a global POI without coupling Richmond
/// generation or presentation to a particular intelligence implementation.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct UrbanSetting {
	pub id: Id,
	pub arrival_radius: f32,
}
