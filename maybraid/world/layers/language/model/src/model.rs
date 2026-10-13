//! [`Language<L>`]: language over world model `L`.

use std::marker::PhantomData;

/// Language model over world `L`.
pub struct Language<L>(PhantomData<fn() -> L>);
