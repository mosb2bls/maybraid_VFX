//! [`Richmond<G>`]: urbanization over ground `G`.

use std::marker::PhantomData;

/// Urbanization model over ground `G`.
pub struct Richmond<G>(PhantomData<fn() -> G>);
