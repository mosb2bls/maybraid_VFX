//! [`Vegetation<V>`]: vegetation over ground `V`.

use std::marker::PhantomData;

/// Vegetation model over ground `V`.
pub struct Vegetation<V>(PhantomData<fn() -> V>);
