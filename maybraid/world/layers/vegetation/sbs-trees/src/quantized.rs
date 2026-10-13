//! Process-wide unit mesh cache keyed by construction type and variant number.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

/// Grow a unit-height plant for `num`, sharing one [`Arc`] for the process.
///
/// The cache key is `(TypeId::of::<Self>(), num)` — not params. `Self` is the
/// construction identity; [`Self::Unit`] is what groves nest (usually the base tree).
/// Remixes of a base model implement this on a wrapper so they do not share the
/// base type's `(num)` slot, then still return `Arc<BaseTree>` for playground hosts.
pub trait QuantizedPlant: Sized + Send + Sync + 'static {
	type Unit: Send + Sync + 'static;

	/// Unitize and grow variant `num`. Called once per construction/number, then cached.
	fn build_unit(num: u32) -> (Self::Unit, f32);

	fn grow_num(num: u32) -> (Arc<Self::Unit>, f32) {
		cached_unit::<Self, Self::Unit, _>(num, || Self::build_unit(num))
	}
}

fn cached_unit<K, T, F>(num: u32, build: F) -> (Arc<T>, f32)
where
	K: 'static,
	T: Send + Sync + 'static,
	F: FnOnce() -> (T, f32),
{
	let cache = UNIT_CACHE.get_or_init(|| RwLock::new(HashMap::new()));
	let key = (TypeId::of::<K>(), num);
	if let Some((arc, size)) = cache.read().expect("unit cache").get(&key) {
		let plant = Arc::downcast::<T>(Arc::clone(arc)).expect("unit cache type");
		return (plant, *size);
	}
	let mut write = cache.write().expect("unit cache");
	if let Some((arc, size)) = write.get(&key) {
		let plant = Arc::downcast::<T>(Arc::clone(arc)).expect("unit cache type");
		return (plant, *size);
	}
	let (unit, size) = build();
	let plant = Arc::new(unit);
	write.insert(key, (Arc::clone(&plant) as Arc<dyn Any + Send + Sync>, size));
	(plant, size)
}

static UNIT_CACHE: OnceLock<RwLock<HashMap<(TypeId, u32), (Arc<dyn Any + Send + Sync>, f32)>>> =
	OnceLock::new();

macro_rules! impl_default_unit {
	($($plant:ty, $params:ty);+ $(;)?) => {
		$(
			impl QuantizedPlant for $plant {
				type Unit = Self;

				fn build_unit(num: u32) -> (Self, f32) {
					let (params, world_size) = <$params>::default().into_unit_from_num(num);
					(params.build(), world_size)
				}
			}
		)+
	};
}

impl_default_unit! {
	crate::BraidOakTree, crate::BraidOakTreeParams;
	crate::DatePalm, crate::DatePalmParams;
	crate::FriendsConifer, crate::FriendsConiferParams;
	crate::HighBushShoots, crate::HighBushShootsParams;
	crate::HonuBanyan, crate::HonuBanyanParams;
	crate::JungleStorybookTree, crate::JungleStorybookTreeParams;
	crate::KamakuraTorch, crate::KamakuraTorchParams;
	crate::LiamsConifer, crate::LiamsConiferParams;
	crate::NorthernConifer, crate::NorthernConiferParams;
	crate::PalmBush, crate::PalmBushParams;
	crate::PenmarchTorch, crate::PenmarchTorchParams;
	crate::RorysHeadTrained, crate::RorysHeadTrainedParams;
	crate::SimplemansHedge, crate::SimplemansHedgeParams;
	crate::SopesBanyan, crate::SopesBanyanParams;
	crate::StorybookTree, crate::StorybookTreeParams;
	crate::TemperateConifer, crate::TemperateConiferParams;
	crate::TuftPatch, crate::TuftPatchParams;
	crate::VaseTree, crate::VaseTreeParams;
	crate::WaialeaPalm, crate::WaialeaPalmParams;
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{StorybookTree, TuftPatch};

	#[test]
	fn grow_num_reuses_arc_for_same_type_and_num() {
		let (a, sa) = StorybookTree::grow_num(3);
		let (b, sb) = StorybookTree::grow_num(3);
		let (c, _) = StorybookTree::grow_num(4);
		assert!(Arc::ptr_eq(&a, &b));
		assert!((sa - sb).abs() < 1e-5);
		assert!(!Arc::ptr_eq(&a, &c));
		assert!((a.geometry.height() - 1.0).abs() < 1e-5);
	}

	#[test]
	fn tuft_grow_num_reuses_arc_for_same_num() {
		let (a, sa) = TuftPatch::grow_num(3);
		let (b, sb) = TuftPatch::grow_num(3);
		let (c, _) = TuftPatch::grow_num(4);
		assert!(Arc::ptr_eq(&a, &b));
		assert!((sa - sb).abs() < 1e-5);
		assert!(!Arc::ptr_eq(&a, &c));
	}
}
