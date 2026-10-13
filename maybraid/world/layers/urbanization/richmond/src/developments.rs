//! [`RichmondDevelopment`]: the development a [`DevelopmentSite`] gets.
//!
//! Every [`Development`] in [`urbanization_developments`] generates the same
//! way, as a [`DevelopmentCell`] over ground `G`; the enum generates the site,
//! then the first of its kinds that fits.

pub mod site;

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use lod::gen::{Id, OriginalId};
use lod::hcsg::{self, GenerationContext};
use procedural_common::Bounds2;
use std::marker::PhantomData;
use urbanization_developments::{
	Development, LesHalles, OldCityMarket, PadParams, PadPlan, RingFort, ShepherdsCommune,
	ShepherdsVillage, SingleHighrise, SiteGround, SkybridgeBazaar, SuburbanHomes, TempleComplex,
	Terrace, WizardsTower,
};

use crate::artifact::BuiltDevelopment;
use crate::config::DevelopmentConfig;
use crate::ground::{GroundCells, RichmondGround};
use crate::pad::{cell_center_xz, PadComplex};
use crate::storage::{column_bounds, overlaps_xz_strictly};

use site::{AuthoredCourtyard, DevelopmentKind, DevelopmentSite};

/// Pad baked from a post-Watershed height sample: flatten terrace + ease skirt.
#[derive(Debug, Clone)]
pub struct DevelopmentPad {
	pub height: f32,
	pub complex: PadComplex,
}

impl From<&PadPlan> for DevelopmentPad {
	fn from(plan: &PadPlan) -> Self {
		Self { height: plan.height, complex: PadComplex::from(plan) }
	}
}

/// A [`Development`] a [`DevelopmentSite`] can be.
pub trait SiteDevelopment: Development {
	const KIND: DevelopmentKind;

	fn into_built(self) -> BuiltDevelopment;
}

macro_rules! site_developments {
	($($development:ident),* $(,)?) => {$(
		impl SiteDevelopment for $development {
			const KIND: DevelopmentKind = DevelopmentKind::$development;

			fn into_built(self) -> BuiltDevelopment {
				BuiltDevelopment::$development(Box::new(self))
			}
		}
	)*};
}

site_developments!(
	LesHalles,
	ShepherdsVillage,
	ShepherdsCommune,
	RingFort,
	TempleComplex,
	SingleHighrise,
	SuburbanHomes,
	WizardsTower,
	SkybridgeBazaar,
	OldCityMarket,
);

/// Development `D` planned on one site over ground `G`.
pub struct DevelopmentCell<D: Development, G> {
	pub cell: Aabb3d,
	pub plan: D::Plan,
	pub pads: Vec<DevelopmentPad>,
	_ground: PhantomData<fn() -> G>,
}

impl<D: SiteDevelopment, G> DevelopmentCell<D, G> {
	pub fn built(&self) -> Option<BuiltDevelopment> {
		D::build(&self.plan).map(D::into_built)
	}

	fn plan(
		site: &DevelopmentSite,
		config: &DevelopmentConfig,
		ground: &mut impl SiteGround,
	) -> Option<Self> {
		let (plan, pads) = D::plan(ground, site.cell, config.seed)?;
		let pads = pads.iter().map(DevelopmentPad::from).collect();
		Some(Self { cell: site.cell, plan, pads, _ground: PhantomData })
	}
}

/// One development cell over ground `G`.
pub enum RichmondDevelopment<G> {
	Empty(Aabb3d),
	LesHalles(DevelopmentCell<LesHalles, G>),
	ShepherdsVillage(DevelopmentCell<ShepherdsVillage, G>),
	ShepherdsCommune(DevelopmentCell<ShepherdsCommune, G>),
	RingFort(DevelopmentCell<RingFort, G>),
	TempleComplex(DevelopmentCell<TempleComplex, G>),
	SingleHighrise(DevelopmentCell<SingleHighrise, G>),
	SuburbanHomes(DevelopmentCell<SuburbanHomes, G>),
	WizardsTower(DevelopmentCell<WizardsTower, G>),
	SkybridgeBazaar(DevelopmentCell<SkybridgeBazaar, G>),
	OldCityMarket(DevelopmentCell<OldCityMarket, G>),
}

impl<G: RichmondGround> hcsg::GenerationScheme for RichmondDevelopment<G> {
	lod::hcsg_index_scale!(crate::storage::COLUMN_INDEX_SCALE);

	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cx.original_ids_for::<DevelopmentSite>(region)
	}

	/// An authored site plans on its own level, dry ground; a procedural one
	/// on `G`, and has no development without ground under its center.
	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let site = cx.get_or_generate::<DevelopmentSite>(id)?;
		let bounds = column_bounds(site.cell);
		if !site.is_filled() {
			return Some((Self::Empty(site.cell), bounds));
		}
		if let Some(authored) = &site.authored {
			let fitted = Self::fit(&site, &authored.config, &mut authored.clone());
			return Some((fitted, bounds));
		}
		let config = cx.get::<DevelopmentConfig>(Id::Universal)?;
		let mut ground = GroundCells::<G>::generate(cx, bounds);
		let center = cell_center_xz(site.cell);
		ground.height_at(center.x, center.y)?;
		Some((Self::fit(&site, &config, &mut ground), bounds))
	}
}

impl<G: RichmondGround> RichmondDevelopment<G> {
	/// Pad nodes of the filled `developments` affecting `region`, merged
	/// into one sample-time blend pass.
	///
	/// One complex matters for overlapping pads: sequential modulation
	/// would let later ease skirts smear earlier exact terraces.
	pub fn merge_pads<'a>(
		region: Aabb3d,
		developments: impl IntoIterator<Item = &'a Self>,
	) -> PadComplex {
		let bounds = Bounds2::from_xz(region.min.x, region.min.z, region.max.x, region.max.z);
		let nodes = developments
			.into_iter()
			.filter(|development| {
				development.is_filled() && overlaps_xz_strictly(region, development.cell())
			})
			.flat_map(Self::pad_complexes)
			.flat_map(|complex| complex.pads.iter())
			.filter(|node| node.correction_intersects(bounds))
			.cloned()
			.collect();
		PadComplex::from_nodes(nodes)
	}

	/// The first of `site`'s kinds that fits on `ground`, else [`Self::Empty`].
	/// An authored development must also build, in its courtyard if it has one.
	fn fit(
		site: &DevelopmentSite,
		config: &DevelopmentConfig,
		ground: &mut impl SiteGround,
	) -> Self {
		for kind in site.kinds() {
			let Some(development) = Self::plan_kind(site, config, ground, kind) else {
				continue;
			};
			let Some(authored) = &site.authored else {
				return development;
			};
			let development = match authored.courtyard {
				Some(courtyard) => development.with_authored_courtyard(courtyard),
				None => Some(development),
			};
			if let Some(development) = development.filter(|d| d.built().is_some()) {
				return development;
			}
		}
		Self::Empty(site.cell)
	}

	fn plan_kind(
		site: &DevelopmentSite,
		config: &DevelopmentConfig,
		ground: &mut impl SiteGround,
		kind: DevelopmentKind,
	) -> Option<Self> {
		fn cell<D: SiteDevelopment, G>(
			site: &DevelopmentSite,
			config: &DevelopmentConfig,
			ground: &mut impl SiteGround,
		) -> Option<DevelopmentCell<D, G>> {
			DevelopmentCell::<D, G>::plan(site, config, ground)
		}
		Some(match kind {
			DevelopmentKind::Empty => return None,
			DevelopmentKind::LesHalles => Self::LesHalles(cell(site, config, ground)?),
			DevelopmentKind::ShepherdsVillage => {
				Self::ShepherdsVillage(cell(site, config, ground)?)
			}
			DevelopmentKind::ShepherdsCommune => {
				Self::ShepherdsCommune(cell(site, config, ground)?)
			}
			DevelopmentKind::RingFort => Self::RingFort(cell(site, config, ground)?),
			DevelopmentKind::TempleComplex => Self::TempleComplex(cell(site, config, ground)?),
			DevelopmentKind::SingleHighrise => Self::SingleHighrise(cell(site, config, ground)?),
			DevelopmentKind::SuburbanHomes => Self::SuburbanHomes(cell(site, config, ground)?),
			DevelopmentKind::WizardsTower => Self::WizardsTower(cell(site, config, ground)?),
			DevelopmentKind::SkybridgeBazaar => Self::SkybridgeBazaar(cell(site, config, ground)?),
			DevelopmentKind::OldCityMarket => Self::OldCityMarket(cell(site, config, ground)?),
		})
	}

	/// Re-pads a single-terrace kind as the authored walled courtyard.
	fn with_authored_courtyard(self, courtyard: AuthoredCourtyard) -> Option<Self> {
		let footprint = self.footprint_half_extents()?;
		let half = (footprint + Vec2::splat(courtyard.margin)).min(Vec2::splat(courtyard.max_half))
			+ Vec2::splat(courtyard.overhang);
		self.with_courtyard(half, PadParams { berm: 0.0, ease: courtyard.ease, round: 0.0 })
	}
}

impl<G> RichmondDevelopment<G> {
	pub fn cell(&self) -> Aabb3d {
		match self {
			Self::Empty(cell) => *cell,
			Self::LesHalles(d) => d.cell,
			Self::ShepherdsVillage(d) => d.cell,
			Self::ShepherdsCommune(d) => d.cell,
			Self::RingFort(d) => d.cell,
			Self::TempleComplex(d) => d.cell,
			Self::SingleHighrise(d) => d.cell,
			Self::SuburbanHomes(d) => d.cell,
			Self::WizardsTower(d) => d.cell,
			Self::SkybridgeBazaar(d) => d.cell,
			Self::OldCityMarket(d) => d.cell,
		}
	}

	pub fn kind(&self) -> DevelopmentKind {
		match self {
			Self::Empty(_) => DevelopmentKind::Empty,
			Self::LesHalles(_) => DevelopmentKind::LesHalles,
			Self::ShepherdsVillage(_) => DevelopmentKind::ShepherdsVillage,
			Self::ShepherdsCommune(_) => DevelopmentKind::ShepherdsCommune,
			Self::RingFort(_) => DevelopmentKind::RingFort,
			Self::TempleComplex(_) => DevelopmentKind::TempleComplex,
			Self::SingleHighrise(_) => DevelopmentKind::SingleHighrise,
			Self::SuburbanHomes(_) => DevelopmentKind::SuburbanHomes,
			Self::WizardsTower(_) => DevelopmentKind::WizardsTower,
			Self::SkybridgeBazaar(_) => DevelopmentKind::SkybridgeBazaar,
			Self::OldCityMarket(_) => DevelopmentKind::OldCityMarket,
		}
	}

	pub fn is_filled(&self) -> bool {
		!matches!(self, Self::Empty(_))
	}

	/// The terrace, for kinds fitted to one terrace.
	pub fn terrace(&self) -> Option<&Terrace> {
		match self {
			Self::LesHalles(d) => Some(&d.plan),
			Self::RingFort(d) => Some(&d.plan),
			Self::TempleComplex(d) => Some(&d.plan),
			Self::SingleHighrise(d) => Some(&d.plan),
			Self::SuburbanHomes(d) => Some(&d.plan),
			Self::WizardsTower(d) => Some(&d.plan),
			Self::SkybridgeBazaar(d) => Some(&d.plan),
			Self::Empty(_)
			| Self::ShepherdsVillage(_)
			| Self::ShepherdsCommune(_)
			| Self::OldCityMarket(_) => None,
		}
	}

	pub fn pads(&self) -> &[DevelopmentPad] {
		match self {
			Self::Empty(_) => &[],
			Self::LesHalles(d) => &d.pads,
			Self::ShepherdsVillage(d) => &d.pads,
			Self::ShepherdsCommune(d) => &d.pads,
			Self::RingFort(d) => &d.pads,
			Self::TempleComplex(d) => &d.pads,
			Self::SingleHighrise(d) => &d.pads,
			Self::SuburbanHomes(d) => &d.pads,
			Self::WizardsTower(d) => &d.pads,
			Self::SkybridgeBazaar(d) => &d.pads,
			Self::OldCityMarket(d) => &d.pads,
		}
	}

	fn terrace_pads_mut(&mut self) -> Option<(&Terrace, &mut Vec<DevelopmentPad>)> {
		match self {
			Self::LesHalles(d) => Some((&d.plan, &mut d.pads)),
			Self::RingFort(d) => Some((&d.plan, &mut d.pads)),
			Self::TempleComplex(d) => Some((&d.plan, &mut d.pads)),
			Self::SingleHighrise(d) => Some((&d.plan, &mut d.pads)),
			Self::SuburbanHomes(d) => Some((&d.plan, &mut d.pads)),
			Self::WizardsTower(d) => Some((&d.plan, &mut d.pads)),
			Self::SkybridgeBazaar(d) => Some((&d.plan, &mut d.pads)),
			Self::Empty(_)
			| Self::ShepherdsVillage(_)
			| Self::ShepherdsCommune(_)
			| Self::OldCityMarket(_) => None,
		}
	}

	pub fn pad_complexes(&self) -> impl Iterator<Item = &PadComplex> {
		self.pads().iter().map(|pad| &pad.complex)
	}

	/// World-axis half extents of the yawed building confines.
	pub fn footprint_half_extents(&self) -> Option<Vec2> {
		self.terrace().map(Terrace::footprint_half_extents)
	}

	/// Replace a single-terrace pad with one axis-aligned terrace at the same
	/// height. `None` for kinds whose pads sit at several heights.
	pub fn with_courtyard(mut self, half_extents: Vec2, params: PadParams) -> Option<Self> {
		let (terrace, pads) = self.terrace_pads_mut()?;
		let courtyard =
			PadPlan::building_skirt(terrace.center_xz(), half_extents, 0.0, terrace.height, params);
		*pads = vec![DevelopmentPad::from(&courtyard)];
		Some(self)
	}

	/// The buildings fitted to a filled cell.
	pub fn built(&self) -> Option<BuiltDevelopment> {
		match self {
			Self::Empty(_) => None,
			Self::LesHalles(d) => d.built(),
			Self::ShepherdsVillage(d) => d.built(),
			Self::ShepherdsCommune(d) => d.built(),
			Self::RingFort(d) => d.built(),
			Self::TempleComplex(d) => d.built(),
			Self::SingleHighrise(d) => d.built(),
			Self::SuburbanHomes(d) => d.built(),
			Self::WizardsTower(d) => d.built(),
			Self::SkybridgeBazaar(d) => d.built(),
			Self::OldCityMarket(d) => d.built(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::cell::DevelopmentExtent;
	use crate::config::DevelopmentConfig;

	fn les_halles() -> RichmondDevelopment<()> {
		let cell = DevelopmentExtent::from_cell_index(0, 0).aabb();
		let terrace = Terrace::new::<LesHalles>(cell, 12.0, DevelopmentConfig::default().seed);
		let pads = vec![DevelopmentPad::from(&terrace.pad())];
		RichmondDevelopment::LesHalles(DevelopmentCell {
			cell,
			plan: terrace,
			pads,
			_ground: PhantomData,
		})
	}

	#[test]
	fn terrace_pad_flattens_the_building_center() {
		let development = les_halles();
		let c = cell_center_xz(development.cell());
		let pad = &development.pads()[0].complex;
		assert!((pad.modify_elevation(3.0, c.x, c.y) - 12.0).abs() < 1e-3);
		assert!((pad.modify_elevation(3.0, 400.0, 400.0) - 3.0).abs() < 1e-3);
	}

	#[test]
	fn courtyard_flattens_the_whole_arena_at_the_pad_height() -> anyhow::Result<()> {
		let development = les_halles();
		let half = development
			.footprint_half_extents()
			.ok_or_else(|| anyhow::anyhow!("terrace footprint"))?
			+ Vec2::splat(20.0);
		let walled = development
			.with_courtyard(half, PadParams { berm: 0.0, ease: 16.0, round: 0.0 })
			.ok_or_else(|| anyhow::anyhow!("terrace courtyard"))?;
		let center = cell_center_xz(walled.cell());
		anyhow::ensure!(walled.pads().len() == 1);
		let pad = &walled.pads()[0].complex;
		for corner in [Vec2::new(1.0, 1.0), Vec2::new(-1.0, 1.0), Vec2::new(1.0, -1.0)] {
			let p = center + corner * (half - Vec2::splat(0.5));
			let y = pad.modify_elevation(-30.0, p.x, p.y);
			anyhow::ensure!((y - 12.0).abs() <= 1e-3, "corner {p} at {y}, expected 12");
		}
		Ok(())
	}
}
