use bevy::prelude::*;

use crate::event::CharacterMenuEvent;

/// Bevy plugin for typed character menu rendering.
///
/// Interaction is handled via `MenuButton<E>` components; the playground
/// listens for `CharacterMenuEvent<M>` after applying widget events to menu
/// state.
pub struct CharacterMenuRendererPlugin<M>(std::marker::PhantomData<M>)
where
	M: Clone + Send + Sync + 'static;

impl<M> Default for CharacterMenuRendererPlugin<M>
where
	M: Clone + Send + Sync + 'static,
{
	fn default() -> Self {
		Self(std::marker::PhantomData)
	}
}

impl<M> Plugin for CharacterMenuRendererPlugin<M>
where
	M: Clone + Send + Sync + 'static,
{
	fn build(&self, app: &mut App) {
		app.add_message::<CharacterMenuEvent<M>>();
	}
}
