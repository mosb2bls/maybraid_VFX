//! [`HcsgStorage`]: completed, immutable values shared across threads.

mod diagnostics;
mod eviction;
mod reads;
mod store;
mod writes;

use std::sync::Arc;

use bevy::prelude::Resource;

use store::Registry;

/// Any value HCSG can store.
pub trait HcsgValue: Send + Sync + 'static {}

impl<T: Send + Sync + 'static> HcsgValue for T {}

/// A lock was held elsewhere. Retry next frame; this is never "empty".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Busy;

/// Cloneable handle over one [`NodeStore`](super::node_store::NodeStore) of `Arc<T>` per type.
///
/// The registry lock is held only long enough to clone a store's `Arc`; a
/// store's lock only for lookup and publication. Neither is held while a
/// value is generated. Values never change once published: replacing one
/// publishes a new [`Version`](crate::gen::Version) from a counter shared by every store.
///
/// ## Reads
///
/// Frame and main-thread code must use [`Self::try_entry`], [`Self::try_overlapping`],
/// and [`Self::try_membership_revision`]. They return [`Busy`] while the worker
/// or eviction sweep holds a store lock.
///
/// Blocking reads ([`Self::get`], [`Self::entry`], [`Self::overlapping`],
/// [`Self::membership_revision`], [`Self::contains`]) are for the generation
/// worker (via [`GenerationContext`](super::context::GenerationContext)) and for
/// tests. Outside `lod` they
/// are only available with the `test-support` feature.
#[derive(Resource, Clone, Default)]
pub struct HcsgStorage(Arc<Registry>);
