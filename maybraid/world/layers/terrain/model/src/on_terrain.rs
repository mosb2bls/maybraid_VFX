//! [`OnTerrain`]: the ground surface of a terrain model.

use std::marker::PhantomData;

/// The solid-ground surface of model `T`, as opposed to its other outputs (water).
pub struct OnTerrain<T>(PhantomData<fn() -> T>);
