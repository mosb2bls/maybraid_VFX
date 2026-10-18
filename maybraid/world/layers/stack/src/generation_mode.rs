//! One active generation mode, keyed by type. This crate never names a mode.

use std::any::TypeId;
use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use bevy::ecs::schedule::SystemSet;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::state::state::FreelyMutableState;

/// A marker the world registers as a generation mode. Layers take `M` as a
/// plugin parameter and never name a concrete mode.
pub trait GenerationMode: Send + Sync + 'static {
	/// Stable label for logs and [`Debug`].
	fn name() -> &'static str {
		std::any::type_name::<Self>()
	}
}

/// Which [`GenerationMode`] is live. [`Self::None`] is only the type default;
/// [`GenerationModePlugin::initial`] picks the startup mode.
#[derive(Clone, Copy, Default)]
pub enum ActiveGenerationMode {
	#[default]
	None,
	Mode {
		id: TypeId,
		name: &'static str,
	},
}

impl ActiveGenerationMode {
	pub fn of<M: GenerationMode>() -> Self {
		Self::Mode { id: TypeId::of::<M>(), name: M::name() }
	}

	pub fn is<M: GenerationMode>(self) -> bool {
		matches!(self, Self::Mode { id, .. } if id == TypeId::of::<M>())
	}

	pub fn mode_id(self) -> Option<TypeId> {
		match self {
			Self::Mode { id, .. } => Some(id),
			Self::None => None,
		}
	}
}

impl PartialEq for ActiveGenerationMode {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Self::None, Self::None) => true,
			(Self::Mode { id: left, .. }, Self::Mode { id: right, .. }) => left == right,
			_ => false,
		}
	}
}

impl Eq for ActiveGenerationMode {}

impl Hash for ActiveGenerationMode {
	fn hash<H: Hasher>(&self, state: &mut H) {
		core::mem::discriminant(self).hash(state);
		if let Self::Mode { id, .. } = self {
			id.hash(state);
		}
	}
}

impl Debug for ActiveGenerationMode {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::None => formatter.write_str("None"),
			Self::Mode { name, .. } => formatter.write_str(name),
		}
	}
}

impl States for ActiveGenerationMode {}

impl FreelyMutableState for ActiveGenerationMode {}

/// Systems a mode-specific plugin adds on [`Update`]. The set is only a label;
/// [`GenerationModePlugin`] attaches the run condition.
pub struct GenerationModeSystems<M: GenerationMode>(PhantomData<fn() -> M>);

impl<M: GenerationMode> Default for GenerationModeSystems<M> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<M: GenerationMode> Clone for GenerationModeSystems<M> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<M: GenerationMode> Copy for GenerationModeSystems<M> {}

impl<M: GenerationMode> PartialEq for GenerationModeSystems<M> {
	fn eq(&self, _other: &Self) -> bool {
		true
	}
}

impl<M: GenerationMode> Eq for GenerationModeSystems<M> {}

impl<M: GenerationMode> Hash for GenerationModeSystems<M> {
	fn hash<H: Hasher>(&self, _state: &mut H) {}
}

impl<M: GenerationMode> Debug for GenerationModeSystems<M> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "GenerationModeSystems<{}>", M::name())
	}
}

impl<M: GenerationMode> SystemSet for GenerationModeSystems<M> {
	fn dyn_clone(&self) -> Box<dyn SystemSet> {
		Box::new(*self)
	}
}

/// The [`GenerationMode`] [`GenerationModePlugin::initial`] selected. A second
/// `initial()` for another mode is a programming error; plugin order must not
/// change the startup mode.
#[derive(Resource, Debug)]
struct StartupGenerationMode {
	id: TypeId,
	name: &'static str,
}

/// Initializes [`ActiveGenerationMode`] once and runs [`GenerationModeSystems<M>`]
/// only while `M` is active.
pub struct GenerationModePlugin<M: GenerationMode> {
	initial: bool,
	_mode: PhantomData<fn() -> M>,
}

impl<M: GenerationMode> GenerationModePlugin<M> {
	/// Make `M` the startup mode. Wins regardless of plugin order.
	pub fn initial() -> Self {
		Self { initial: true, _mode: PhantomData }
	}
}

impl<M: GenerationMode> Default for GenerationModePlugin<M> {
	fn default() -> Self {
		Self { initial: false, _mode: PhantomData }
	}
}

impl<M: GenerationMode> Plugin for GenerationModePlugin<M> {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<StatesPlugin>() {
			app.add_plugins(StatesPlugin);
		}
		if !app.world().contains_resource::<State<ActiveGenerationMode>>() {
			app.init_state::<ActiveGenerationMode>();
		}
		if self.initial {
			if let Some(startup) = app.world().get_resource::<StartupGenerationMode>() {
				if startup.id != TypeId::of::<M>() {
					panic!(
						"GenerationModePlugin::initial() already selected {}; cannot also initial {}",
						startup.name,
						M::name()
					);
				}
			} else {
				app.insert_resource(StartupGenerationMode {
					id: TypeId::of::<M>(),
					name: M::name(),
				});
			}
			app.insert_state(ActiveGenerationMode::of::<M>());
		}
		app.configure_sets(
			Update,
			GenerationModeSystems::<M>::default().run_if(in_state(ActiveGenerationMode::of::<M>())),
		);
	}
}

/// True while `M` is the active generation mode.
pub fn in_generation_mode<M: GenerationMode>(
) -> impl FnMut(Option<Res<State<ActiveGenerationMode>>>) -> bool + Clone {
	in_state(ActiveGenerationMode::of::<M>())
}

#[cfg(test)]
mod tests {
	use super::*;

	struct Alpha;
	struct Beta;

	impl GenerationMode for Alpha {}
	impl GenerationMode for Beta {}

	#[derive(Resource, Default, Debug)]
	struct ModeTicks {
		alpha: u32,
		beta: u32,
	}

	fn tick_alpha(mut ticks: ResMut<ModeTicks>) {
		ticks.alpha += 1;
	}

	fn tick_beta(mut ticks: ResMut<ModeTicks>) {
		ticks.beta += 1;
	}

	fn mode_app() -> App {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			StatesPlugin,
			GenerationModePlugin::<Alpha>::initial(),
			GenerationModePlugin::<Beta>::default(),
		));
		app.init_resource::<ModeTicks>();
		app.add_systems(
			Update,
			(
				tick_alpha.in_set(GenerationModeSystems::<Alpha>::default()),
				tick_beta.in_set(GenerationModeSystems::<Beta>::default()),
			),
		);
		app
	}

	#[test]
	fn a_mode_set_runs_only_while_that_mode_is_active() -> anyhow::Result<()> {
		let mut app = mode_app();
		app.update();
		let ticks = app.world().resource::<ModeTicks>();
		anyhow::ensure!(
			ticks.alpha == 1 && ticks.beta == 0,
			"startup must run only the initial mode, ticks {ticks:?}"
		);

		app.world_mut()
			.resource_mut::<NextState<ActiveGenerationMode>>()
			.set(ActiveGenerationMode::of::<Beta>());
		app.update();
		let ticks = app.world().resource::<ModeTicks>();
		anyhow::ensure!(
			ticks.alpha == 1 && ticks.beta == 1,
			"exactly one mode may tick, ticks {ticks:?}"
		);
		app.update();
		let ticks = app.world().resource::<ModeTicks>();
		anyhow::ensure!(
			ticks.alpha == 1 && ticks.beta == 2,
			"the inactive mode must stay idle, ticks {ticks:?}"
		);
		Ok(())
	}

	#[derive(Resource, Default)]
	struct TransitionCounts {
		entered: u32,
		exited: u32,
	}

	fn count_enter(mut counts: ResMut<TransitionCounts>) {
		counts.entered += 1;
	}

	fn count_exit(mut counts: ResMut<TransitionCounts>) {
		counts.exited += 1;
	}

	#[test]
	fn setting_the_same_mode_reenters_and_set_if_neq_does_not() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			StatesPlugin,
			GenerationModePlugin::<Alpha>::initial(),
			GenerationModePlugin::<Beta>::default(),
		));
		app.init_resource::<TransitionCounts>();
		app.add_systems(OnEnter(ActiveGenerationMode::of::<Beta>()), count_enter);
		app.add_systems(OnExit(ActiveGenerationMode::of::<Beta>()), count_exit);
		app.update();
		anyhow::ensure!(
			app.world().resource::<TransitionCounts>().entered == 0,
			"the initial mode must not enter the other mode"
		);

		app.world_mut()
			.resource_mut::<NextState<ActiveGenerationMode>>()
			.set(ActiveGenerationMode::of::<Beta>());
		app.update();
		let counts = app.world().resource::<TransitionCounts>();
		anyhow::ensure!(
			counts.entered == 1 && counts.exited == 0,
			"the first enter runs OnEnter only"
		);

		app.world_mut()
			.resource_mut::<NextState<ActiveGenerationMode>>()
			.set(ActiveGenerationMode::of::<Beta>());
		app.update();
		let counts = app.world().resource::<TransitionCounts>();
		anyhow::ensure!(
			counts.entered == 2 && counts.exited == 1,
			"NextState::set of the current mode runs OnExit and OnEnter"
		);

		{
			let mut next = app.world_mut().resource_mut::<NextState<ActiveGenerationMode>>();
			NextState::set_if_neq(&mut next, ActiveGenerationMode::of::<Beta>());
		}
		app.update();
		let counts = app.world().resource::<TransitionCounts>();
		anyhow::ensure!(
			counts.entered == 2 && counts.exited == 1,
			"set_if_neq of the current mode does not re-enter"
		);
		Ok(())
	}

	fn current_mode(app: &App) -> ActiveGenerationMode {
		*app.world().resource::<State<ActiveGenerationMode>>().get()
	}

	#[test]
	fn initial_wins_regardless_of_plugin_order() -> anyhow::Result<()> {
		let mut after_default = App::new();
		after_default.add_plugins((
			MinimalPlugins,
			StatesPlugin,
			GenerationModePlugin::<Beta>::default(),
			GenerationModePlugin::<Alpha>::initial(),
		));
		after_default.update();
		anyhow::ensure!(
			current_mode(&after_default).is::<Alpha>(),
			"initial after default started as {:?}",
			current_mode(&after_default)
		);

		let mut after_initial = App::new();
		after_initial.add_plugins((
			MinimalPlugins,
			StatesPlugin,
			GenerationModePlugin::<Alpha>::initial(),
			GenerationModePlugin::<Beta>::default(),
		));
		after_initial.update();
		anyhow::ensure!(
			current_mode(&after_initial).is::<Alpha>(),
			"initial before default started as {:?}",
			current_mode(&after_initial)
		);
		Ok(())
	}

	#[test]
	fn a_second_initial_mode_fails() -> anyhow::Result<()> {
		let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
			let mut app = App::new();
			app.add_plugins((
				MinimalPlugins,
				StatesPlugin,
				GenerationModePlugin::<Alpha>::initial(),
				GenerationModePlugin::<Beta>::initial(),
			));
		}));
		anyhow::ensure!(failed.is_err(), "a second initial() must fail loudly");
		Ok(())
	}
}
