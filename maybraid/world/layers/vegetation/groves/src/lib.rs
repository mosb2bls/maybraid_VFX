//! Cellular grove selection for Chico vegetation ([RFC-183 §4.7](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation#47-grove-core-selection)).
//!
//! Crate-root re-exports are the built grove and its params (`Orchard`, `OrchardParams`).
//! Cell / item / authored geometry types stay on the grove module
//! (`vegetation_groves::orchard::OrchardCell`). [`OasisDatePalm`] is also at the root because
//! playground hosts register that plant family.

pub mod grove;

pub mod alpine;
pub mod arid_conifer_sapling;
pub mod braid_grass;
pub mod bush_scrub;
pub mod christmas_taiga;
pub mod common_tufts;
pub mod conifer_massives;
pub mod conifer_sapling;
pub mod date_grove;
pub mod dryland;
pub mod forlorn_savanna;
pub mod goettingen_follow;
pub mod high_bush;
pub mod jerrys_chaparral;
pub mod jungle_lower_massives;
pub mod jungle_massives;
pub mod leeward;
pub mod levantine_scrub;
pub mod low_bush;
pub mod monster_grass;
pub mod orchard;
pub mod palm_shade;
pub mod riparian_general;
pub mod riparian_mix;
pub mod riverine_green;
pub mod rolling_oaks;
pub mod shamanhome;
pub mod spotty_bushes;
pub mod storytellers;
pub mod strange_oasis;
pub mod tall_grass;
pub mod temperate_lower_massives;
pub mod temperate_massives;
pub mod trade_winds;
pub mod tropical_thicket;
pub mod tropical_tufts;
pub mod tropical_undergrowth;
pub mod unending_jungle;
pub mod vineyard;
pub mod wandering_acacia;
pub mod wild_grass;

pub use grove::{
	cell_center, parse_variant_weights, parse_vec2_csv, parse_vec3_csv, placement_noise,
	FlatTerrainSample, FnHeightSample, ForestGroveBiases, Grove, GroveBucket, GroveCellOutcome,
	GroveCellVariant, GroveDefinition, GroveDistribution, GroveExtent, GroveFrontend,
	GroveHeightModulation, GroveHeightModulationStack, GrovePlacementRanges, GroveRecipe,
	GroveTerrain, GroveWorldSample, ModulatedGroveSample, PaletteColor, PaletteMix, PaletteSlot,
	PlacementConstraints, PlacementSample, PreparedGroveDistribution, TerrainGroveSample,
	VariantWeightOverrides, DEFAULT_GROVE_EXTENT_XZ,
};

#[cfg(feature = "render")]
pub use grove::{
	patch_spawned_leaf_material, resolve_palette_color, GrovePreviewParams, WithPalette,
	WoodyCanopyPolicy, WoodyGroveLod,
};

#[cfg(feature = "render")]
pub use alpine::{Alpine, AlpineParams};
#[cfg(feature = "render")]
pub use arid_conifer_sapling::{AridConiferSapling, AridConiferSaplingParams};
#[cfg(feature = "render")]
pub use braid_grass::{BraidGrass, BraidGrassParams};
#[cfg(feature = "render")]
pub use bush_scrub::{BushScrub, BushScrubParams};
#[cfg(feature = "render")]
pub use christmas_taiga::{ChristmasTaiga, ChristmasTaigaParams};
#[cfg(feature = "render")]
pub use common_tufts::{CommonTufts, CommonTuftsParams};
#[cfg(feature = "render")]
pub use conifer_massives::{ConiferMassives, ConiferMassivesParams};
#[cfg(feature = "render")]
pub use conifer_sapling::{ConiferSapling, ConiferSaplingParams};
#[cfg(feature = "render")]
pub use date_grove::{DateGrove, DateGroveParams};
#[cfg(feature = "render")]
pub use dryland::{Dryland, DrylandParams};
#[cfg(feature = "render")]
pub use forlorn_savanna::{ForlornSavanna, ForlornSavannaParams};
#[cfg(feature = "render")]
pub use goettingen_follow::{GoettingenFollow, GoettingenFollowParams};
#[cfg(feature = "render")]
pub use high_bush::{HighBush, HighBushParams};
#[cfg(feature = "render")]
pub use jerrys_chaparral::{JerrysChaparral, JerrysChaparralParams};
#[cfg(feature = "render")]
pub use jungle_lower_massives::{JungleLowerMassives, JungleLowerMassivesParams};
#[cfg(feature = "render")]
pub use jungle_massives::{JungleMassives, JungleMassivesParams};
#[cfg(feature = "render")]
pub use leeward::{Leeward, LeewardParams};
#[cfg(feature = "render")]
pub use levantine_scrub::{LevantineScrub, LevantineScrubParams};
#[cfg(feature = "render")]
pub use low_bush::{LowBush, LowBushParams};
#[cfg(feature = "render")]
pub use monster_grass::{MonsterGrass, MonsterGrassParams};
#[cfg(feature = "render")]
pub use orchard::{Orchard, OrchardParams};
#[cfg(feature = "render")]
pub use palm_shade::{PalmShade, PalmShadeParams};
#[cfg(feature = "render")]
pub use riparian_general::{RiparianGeneral, RiparianGeneralParams};
#[cfg(feature = "render")]
pub use riparian_mix::{RiparianMix, RiparianMixParams};
#[cfg(feature = "render")]
pub use riverine_green::{RiverineGreen, RiverineGreenParams};
#[cfg(feature = "render")]
pub use rolling_oaks::{RollingOaks, RollingOaksParams};
#[cfg(feature = "render")]
pub use shamanhome::{Shamanhome, ShamanhomeParams};
#[cfg(feature = "render")]
pub use spotty_bushes::{SpottyBushes, SpottyBushesParams};
#[cfg(feature = "render")]
pub use storytellers::{Storytellers, StorytellersParams};
#[cfg(feature = "render")]
pub use strange_oasis::{OasisDatePalm, StrangeOasis, StrangeOasisParams};
#[cfg(feature = "render")]
pub use tall_grass::{TallGrass, TallGrassParams};
#[cfg(feature = "render")]
pub use temperate_lower_massives::{TemperateLowerMassives, TemperateLowerMassivesParams};
#[cfg(feature = "render")]
pub use temperate_massives::{TemperateMassives, TemperateMassivesParams};
#[cfg(feature = "render")]
pub use trade_winds::{TradeWinds, TradeWindsParams};
#[cfg(feature = "render")]
pub use tropical_thicket::{TropicalThicket, TropicalThicketParams};
#[cfg(feature = "render")]
pub use tropical_tufts::{TropicalTufts, TropicalTuftsParams};
#[cfg(feature = "render")]
pub use tropical_undergrowth::{TropicalUndergrowth, TropicalUndergrowthParams};
#[cfg(feature = "render")]
pub use unending_jungle::{UnendingJungle, UnendingJungleParams};
#[cfg(feature = "render")]
pub use vineyard::{Vineyard, VineyardParams};
#[cfg(feature = "render")]
pub use wandering_acacia::{WanderingAcacia, WanderingAcaciaParams};
#[cfg(feature = "render")]
pub use wild_grass::{WildGrass, WildGrassParams};
