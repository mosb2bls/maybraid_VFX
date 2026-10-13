//! [`Mobs<B>`]: mobs over ground `B`.

use std::marker::PhantomData;

/// Mob model over ground `B`.
pub struct Mobs<B>(PhantomData<fn() -> B>);
