//! [`generation<C, T>`]: keeps `T` warm in storage within channel `C`'s regions.

use std::marker::PhantomData;

use bevy::prelude::{App, IntoScheduleConfigs, Local, MessageReader, Plugin, Res, Update};

use super::bounds::HcsgRegions;
use super::context::GenerationScheme;
use super::demand::HcsgDemand;
use super::request::HcsgRequest;
use super::runtime::{ensure_runtime, HcsgSystems};

/// Per-system request state for [`generation`].
pub struct Generated<T> {
	request: HcsgRequest,
	_t: PhantomData<fn() -> T>,
}

impl<T> Default for Generated<T> {
	fn default() -> Self {
		Self { request: HcsgRequest::default(), _t: PhantomData }
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
		state.request.apply_regions(latest);
	}
	if state.request.sync::<T>(&demand).is_err() {
		return;
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
