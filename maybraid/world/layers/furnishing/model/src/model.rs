//! [`Furnishing<F>`]: furnishing over ground `F`.

use std::marker::PhantomData;

/// Furnishing model over ground `F`.
pub struct Furnishing<F>(PhantomData<fn() -> F>);
