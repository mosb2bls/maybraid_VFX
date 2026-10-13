//! Shared BSN helpers for composing child scenes.

use bevy::ecs::component::Component;
use bevy::prelude::{Children, Transform, Visibility};
use bevy::scene::prelude::{bsn, template_value, Scene};

/// Group child scenes under a hierarchy-safe root.
pub fn scene_children(children: Vec<Box<dyn Scene>>) -> impl Scene + 'static {
	bsn! {
		Transform::default()
		Visibility::default()
		Children [ {children} ]
	}
}

/// Overlay an optional component onto the current entity (no extra child).
pub fn maybe_component<C>(value: Option<C>) -> Box<dyn Scene>
where
	C: Component + Clone + Default + Unpin,
{
	match value {
		Some(c) => Box::new(bsn! {
			template_value(c)
		}),
		None => Box::new(bsn! {
			Visibility::Inherited
		}),
	}
}
