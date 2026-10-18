//! [`Urbanization<U>`]: pads and urban artifacts composed over ground `U`.

use std::marker::PhantomData;

/// Model `U` after urbanization: pads composed into `U`'s ground surface.
pub struct Urbanization<U>(PhantomData<fn() -> U>);
