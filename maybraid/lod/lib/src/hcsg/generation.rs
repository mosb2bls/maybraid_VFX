//! [`generation<C, T>`]: keeps `T` warm in storage within channel `C`'s regions.

use std::marker::PhantomData;

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use bevy::prelude::{App, IntoScheduleConfigs, Local, MessageReader, Plugin, Res, Update};

use super::bounds::{HcsgClass, HcsgRegions};
use super::context::GenerationScheme;
use super::demand::{HcsgDemand, SubscriptionId};
use super::runtime::{ensure_runtime, HcsgSystems};

/// Per-system request state for [`generation`].
pub struct Generated<T> {
	/// The channel's latest regions.
	wanted: Vec<Aabb3d>,
	focus: Option<Vec3>,
	class: HcsgClass,
	subscription: Option<SubscriptionId>,
	requested: Vec<Aabb3d>,
	_t: PhantomData<fn() -> T>,
}

impl<T> Default for Generated<T> {
	fn default() -> Self {
		Self {
			wanted: Vec::new(),
			focus: None,
			class: HcsgClass::Near,
			subscription: None,
			requested: Vec::new(),
			_t: PhantomData,
		}
	}
}

/// Fire and forget: replaces its subscription when `C`'s regions change or
/// the subscription is dropped (an epoch change). Never reads results.
pub fn generation<C: Send + Sync + 'static, T: GenerationScheme>(
	mut regions: MessageReader<HcsgRegions<C>>,
	demand: Res<HcsgDemand>,
	mut state: Local<Generated<T>>,
) {
	if let Some(latest) = regions.read().last() {
		state.wanted = latest.boxes.clone();
		state.focus = latest.focus;
		state.class = latest.class;
	}
	let live = match state.subscription {
		Some(subscription) => match demand.try_is_live(subscription) {
			Ok(live) => live,
			Err(_) => return,
		},
		None => false,
	};
	let wants = !state.wanted.is_empty();
	if state.wanted == state.requested && live == wants {
		return;
	}
	state.requested = state.wanted.clone();
	if wants {
		let regions = state.wanted.clone();
		let focus = state.focus;
		state.subscription =
			Some(demand.subscribe::<T>(state.subscription, regions, focus, state.class));
	} else if let Some(subscription) = state.subscription.take() {
		demand.unsubscribe(subscription);
	}
}

/// Keeps `T` generated within channel `C`'s regions without presenting it. A
/// presented type needs none: presentation requests its own generation.
pub struct GenerationPlugin<C, T>(PhantomData<fn() -> (C, T)>);

impl<C, T> Default for GenerationPlugin<C, T> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<C: Send + Sync + 'static, T: GenerationScheme> Plugin for GenerationPlugin<C, T> {
	fn build(&self, app: &mut App) {
		ensure_runtime(app);
		app.add_message::<HcsgRegions<C>>()
			.add_systems(Update, generation::<C, T>.in_set(HcsgSystems));
	}
}
