//! Menu option trait implementations for item types.

use bevy_math::Vec3;
use character_ui_menu::{
	AssetOption, IdentifiedAsset, LabelOption, ListValues, StringIdentified, SwatchOption,
	ThumbnailCamera,
};

use crate::{ClothingMaterial, ClothingMesh, FirearmMesh, ItemColor};

const CLOTHING_THUMBNAIL_CAMERA: ThumbnailCamera =
	ThumbnailCamera::new(Vec3::new(0.0, 0.75, 2.5), Vec3::new(0.0, 0.65, 0.0));

const FIREARM_THUMBNAIL_CAMERA: ThumbnailCamera =
	ThumbnailCamera::new(Vec3::new(0.0, 0.35, 5.0), Vec3::new(0.0, 0.15, 0.0));

macro_rules! impl_menu_identity {
	($ty:ty) => {
		impl ListValues for $ty {
			fn values() -> &'static [Self] {
				Self::VALUES
			}
		}

		impl LabelOption for $ty {
			fn label(&self) -> &'static str {
				(*self).label()
			}
		}

		impl StringIdentified for $ty {
			fn id(&self) -> &'static str {
				(*self).label()
			}
		}
	};
}

impl_menu_identity!(ClothingMesh);
impl_menu_identity!(ClothingMaterial);
impl_menu_identity!(FirearmMesh);
impl_menu_identity!(ItemColor);

impl AssetOption for ClothingMesh {
	fn asset(&self) -> IdentifiedAsset {
		IdentifiedAsset::new((*self).label(), (*self).label(), (*self).path())
			.with_thumbnail_camera(CLOTHING_THUMBNAIL_CAMERA)
	}
}

impl AssetOption for FirearmMesh {
	fn asset(&self) -> IdentifiedAsset {
		IdentifiedAsset::new((*self).label(), (*self).label(), (*self).path())
			.with_thumbnail_camera(FIREARM_THUMBNAIL_CAMERA)
	}
}

impl SwatchOption for ItemColor {
	fn color_hex(&self) -> &'static str {
		(*self).color_hex()
	}
}
