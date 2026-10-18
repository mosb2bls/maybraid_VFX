//! Explicit English naming vocabulary. Variant Debug text is not a source.

use chico::{ForestGroveKind, LayeringKind};
use durham::GeographicFeatureKind;
use maybraid_language_core::lexicalizer::mix;
use richmond::DiscoverablePlaceLabel;
use urbanization_cells::{UrbanDevelopmentKind, UrbanizationKind};

/// Modifier adjectives plus a required noun head.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KindVocab {
	pub modifiers: &'static [&'static str],
	pub heads: &'static [&'static str],
}

impl KindVocab {
	const fn new(modifiers: &'static [&'static str], heads: &'static [&'static str]) -> Self {
		Self { modifiers, heads }
	}

	const fn heads_only(heads: &'static [&'static str]) -> Self {
		Self { modifiers: &[], heads }
	}
}

/// Seeded phrase: optional color, one modifier synonym, one required noun.
pub fn compose_english(modifiers: &[&str], heads: &[&str], seed: u64, color: bool) -> Vec<String> {
	compose_owned(&unique_keep_order(modifiers), &unique_keep_order(heads), seed, color)
}

fn compose_owned(modifiers: &[String], heads: &[String], seed: u64, color: bool) -> Vec<String> {
	if heads.is_empty() && modifiers.is_empty() {
		return Vec::new();
	}
	let head = if heads.is_empty() {
		pick_word(modifiers, seed ^ 0x51ED)
	} else {
		pick_word(heads, seed ^ 0x51ED)
	};
	let extras: Vec<String> = modifiers.iter().filter(|word| *word != &head).cloned().collect();
	let modifier = (!extras.is_empty()).then(|| pick_word(&extras, seed ^ 0xA11A));
	let mut out = Vec::new();
	if color {
		out.push(color_term(seed));
	}
	if let Some(word) = modifier {
		out.push(word);
	}
	if !out.iter().any(|word| word == &head) {
		out.push(head);
	}
	out
}

/// Color + one grove descriptor + a grove noun (`Blue Riparian Grove`).
pub fn named_grove_english(
	kinds: impl IntoIterator<Item = ForestGroveKind>,
	seed: u64,
) -> Vec<String> {
	let (modifiers, heads) = merge_vocabs(kinds.into_iter().map(grove_vocab));
	compose_owned(&modifiers, &heads, seed, true)
}

/// Forest-scale phrase without a color (`Riparian Woods`).
pub fn named_forest_english(
	layering: LayeringKind,
	kinds: impl IntoIterator<Item = ForestGroveKind>,
	seed: u64,
) -> Vec<String> {
	let (modifiers, heads) = merge_vocabs(
		std::iter::once(layering_vocab(layering)).chain(kinds.into_iter().map(grove_vocab)),
	);
	compose_owned(&modifiers, &heads, seed, false)
}

/// One geographic noun, with a descriptor when the kind is only an adjective.
pub fn named_geographic_english(kind: GeographicFeatureKind, seed: u64) -> Vec<String> {
	let vocab = geographic_vocab(kind);
	compose_english(vocab.modifiers, vocab.heads, seed, false)
}

/// Urban cell or leaf: keep a settlement or building noun.
pub fn named_urban_english(
	kind: UrbanizationKind,
	leaf: Option<UrbanDevelopmentKind>,
	seed: u64,
) -> Vec<String> {
	let mut vocabs = vec![urbanization_vocab(kind)];
	if let Some(leaf) = leaf {
		vocabs.push(development_vocab(leaf));
	}
	let (modifiers, heads) = merge_vocabs(vocabs);
	compose_owned(&modifiers, &heads, seed, false)
}

/// Seeded region phrase. Independent of streamed groves and towns.
pub fn named_region_english(world_seed: u64, ix: i32, iz: i32) -> Vec<String> {
	compose_english(
		&["high", "low", "old", "far", "near", "great", "little"],
		&["land", "country", "march", "reach", "vale", "downs", "heath", "moor"],
		mix(world_seed ^ mix(ix as u64) ^ mix((iz as u64).wrapping_mul(17)) ^ 0x51A7),
		false,
	)
}

/// Color + a place noun (`Amber Lodge`).
pub fn named_place_english(label: DiscoverablePlaceLabel, seed: u64) -> Vec<String> {
	let vocab = place_vocab(label);
	compose_english(vocab.modifiers, vocab.heads, seed, true)
}

/// Terms Geneva may pick from a Durham geographic kind.
pub fn geographic_terms(kind: GeographicFeatureKind) -> Vec<String> {
	flatten(geographic_vocab(kind))
}

/// Terms implied by a forest layering. Internal recipe names stay out.
pub fn layering_terms(kind: LayeringKind) -> Vec<String> {
	flatten(layering_vocab(kind))
}

/// Terms implied by a selected grove. Implementation variants collapse.
pub fn grove_kind_terms(kind: ForestGroveKind) -> Vec<String> {
	flatten(grove_vocab(kind))
}

/// Terms implied by an urbanization cell kind.
pub fn urbanization_terms(kind: UrbanizationKind) -> Vec<String> {
	flatten(urbanization_vocab(kind))
}

/// Terms implied by a development leaf.
pub fn development_terms(kind: UrbanDevelopmentKind) -> Vec<String> {
	flatten(development_vocab(kind))
}

/// Terms implied by a discoverable place label.
pub fn place_label_terms(label: DiscoverablePlaceLabel) -> Vec<String> {
	flatten(place_vocab(label))
}

fn geographic_vocab(kind: GeographicFeatureKind) -> KindVocab {
	match kind {
		GeographicFeatureKind::Massif => KindVocab::new(
			&["high", "craggy", "rugged"],
			&["massif", "mountain", "range", "peak", "heights", "ridge"],
		),
		GeographicFeatureKind::Plateau => KindVocab::new(
			&["high", "open", "broad"],
			&["plateau", "tableland", "mesa", "highland", "upland"],
		),
		GeographicFeatureKind::Canyon => KindVocab::new(
			&["deep", "cut", "narrow"],
			&["canyon", "gorge", "ravine", "gulch", "chasm"],
		),
		GeographicFeatureKind::Rolling => KindVocab::new(
			&["rolling", "undulating", "gentle"],
			&["hills", "downs", "knolls", "rises"],
		),
		GeographicFeatureKind::Valley => {
			KindVocab::new(&["low", "sheltered", "green"], &["valley", "vale", "glen", "dale"])
		}
		GeographicFeatureKind::PocketWater => KindVocab::new(
			&["hidden", "still", "quiet"],
			&["pool", "pond", "tarn", "mere", "waterhole"],
		),
		GeographicFeatureKind::Lake => {
			KindVocab::new(&["wide", "still", "clear"], &["lake", "loch", "tarn", "mere", "waters"])
		}
		GeographicFeatureKind::Bog => {
			KindVocab::new(&["wet", "sinking", "soft"], &["bog", "marsh", "fen", "mire", "swamp"])
		}
		GeographicFeatureKind::Stream | GeographicFeatureKind::StreamsGraph => KindVocab::new(
			&["running", "winding", "clear"],
			&["stream", "brook", "creek", "rivulet", "run"],
		),
	}
}

fn layering_vocab(kind: LayeringKind) -> KindVocab {
	match kind {
		LayeringKind::LushJungle | LayeringKind::Kumulipo => KindVocab::new(
			&["lush", "tangled", "deep"],
			&["jungle", "rainforest", "wilds", "tangle"],
		),
		LayeringKind::Riparian => KindVocab::new(
			&["riparian", "riverside", "riverine"],
			&["woods", "forest", "gallery", "woodland"],
		),
		LayeringKind::Taiga => {
			KindVocab::new(&["boreal", "northern"], &["taiga", "woodland", "forest", "woods"])
		}
		LayeringKind::LiamsSummer => KindVocab::new(
			&["summer", "green", "mild"],
			&["woods", "forest", "woodland", "country"],
		),
		LayeringKind::OwlsDesert | LayeringKind::OldNevada | LayeringKind::DamasEdge => {
			KindVocab::new(
				&["arid", "dry", "sunstruck"],
				&["desert", "scrubland", "waste", "expanse"],
			)
		}
		LayeringKind::MiRobles => {
			KindVocab::new(&["oak", "woody"], &["woods", "forest", "woodland", "groves"])
		}
		LayeringKind::Seceda => {
			KindVocab::new(&["alpine", "high"], &["woods", "forest", "slopes", "highland"])
		}
		LayeringKind::Waiguo | LayeringKind::OldSteppe | LayeringKind::SteppeDown => {
			KindVocab::new(&["open", "windswept"], &["steppe", "grassland", "plain", "prairie"])
		}
		LayeringKind::AgTown | LayeringKind::Storybook | LayeringKind::FruitPlains => {
			KindVocab::new(&["fruiting", "tended"], &["orchard", "plantings", "grove", "gardens"])
		}
		LayeringKind::SunsBarren => {
			KindVocab::new(&["barren", "bare", "sparse"], &["ground", "flats", "waste", "heath"])
		}
		LayeringKind::TemperateHoly => KindVocab::new(
			&["temperate", "quiet", "hallowed"],
			&["woods", "forest", "woodland", "grove"],
		),
		LayeringKind::TrapThicket => {
			KindVocab::new(&["dense", "tangled"], &["thicket", "brake", "copse", "scrub"])
		}
		LayeringKind::Bush => {
			KindVocab::new(&["low", "scrubby"], &["bush", "scrub", "thicket", "brush"])
		}
		LayeringKind::Meadowland => {
			KindVocab::new(&["open", "flowering"], &["meadow", "lea", "pasture", "field"])
		}
		LayeringKind::OpenTropics | LayeringKind::WestMaui => KindVocab::new(
			&["tropical", "warm", "open"],
			&["tropics", "grove", "woodland", "coast"],
		),
		LayeringKind::UpperPark => {
			KindVocab::new(&["tended", "open"], &["park", "green", "grounds", "common"])
		}
	}
}

fn grove_vocab(kind: ForestGroveKind) -> KindVocab {
	match kind {
		ForestGroveKind::Alpine => {
			KindVocab::new(&["alpine", "high", "cold"], &["grove", "stand", "woods", "wood"])
		}
		ForestGroveKind::AridConiferSapling
		| ForestGroveKind::ConiferSapling
		| ForestGroveKind::ConiferMassives => KindVocab::new(
			&["conifer", "evergreen", "needle"],
			&["grove", "stand", "woods", "forest"],
		),
		ForestGroveKind::BraidGrass
		| ForestGroveKind::MonsterGrass
		| ForestGroveKind::TallGrass
		| ForestGroveKind::WildGrass => {
			KindVocab::new(&["wild", "tall", "waving"], &["grass", "grassland", "meadow", "lea"])
		}
		ForestGroveKind::BushScrub
		| ForestGroveKind::HighBush
		| ForestGroveKind::LowBush
		| ForestGroveKind::SpottyBushes => {
			KindVocab::new(&["low", "scrubby", "spotty"], &["bush", "thicket", "scrub", "brush"])
		}
		ForestGroveKind::ChristmasTaiga => {
			KindVocab::new(&["boreal", "snowy"], &["taiga", "woods", "forest", "stand"])
		}
		ForestGroveKind::CommonTufts | ForestGroveKind::TropicalTufts => {
			KindVocab::new(&["tufted", "clumped"], &["tufts", "sward", "grass", "turf"])
		}
		ForestGroveKind::DateGrove => {
			KindVocab::new(&["date", "fruiting"], &["grove", "stand", "garden", "palmery"])
		}
		ForestGroveKind::Dryland => {
			KindVocab::new(&["dry", "arid", "parched"], &["flat", "country", "scrub", "ground"])
		}
		ForestGroveKind::ForlornSavanna => {
			KindVocab::new(&["forlorn", "open"], &["savanna", "veld", "grassland", "plain"])
		}
		ForestGroveKind::GoettingenFollow
		| ForestGroveKind::Shamanhome
		| ForestGroveKind::Storytellers => {
			KindVocab::new(&["old", "quiet", "storied"], &["grove", "stand", "woods", "glade"])
		}
		ForestGroveKind::JerrysChaparral => {
			KindVocab::new(&["dry", "hardy"], &["chaparral", "scrub", "brush", "thicket"])
		}
		ForestGroveKind::JungleLowerMassives
		| ForestGroveKind::JungleMassives
		| ForestGroveKind::UnendingJungle
		| ForestGroveKind::TropicalUndergrowth => KindVocab::new(
			&["tropical", "dense", "tangled"],
			&["jungle", "rainforest", "wilds", "tangle"],
		),
		ForestGroveKind::Leeward => {
			KindVocab::new(&["leeward", "sheltered"], &["grove", "stand", "woods", "slope"])
		}
		ForestGroveKind::LevantineScrub => {
			KindVocab::new(&["dry", "low"], &["scrub", "brush", "thicket", "heath"])
		}
		ForestGroveKind::Orchard => {
			KindVocab::new(&["fruiting", "tended"], &["orchard", "plantings", "grove", "garden"])
		}
		ForestGroveKind::PalmShade => {
			KindVocab::new(&["palm", "shady"], &["grove", "stand", "shade", "palmery"])
		}
		ForestGroveKind::RiparianGeneral | ForestGroveKind::RiparianMix => KindVocab::new(
			&["riparian", "riverside", "riverine"],
			&["grove", "stand", "gallery", "woods"],
		),
		ForestGroveKind::RiverineGreen => {
			KindVocab::new(&["riverine", "green", "watered"], &["grove", "stand", "bank", "woods"])
		}
		ForestGroveKind::RollingOaks => {
			KindVocab::new(&["oak", "rolling"], &["grove", "stand", "copse", "woods"])
		}
		ForestGroveKind::StrangeOasis => {
			KindVocab::new(&["strange", "hidden"], &["oasis", "spring", "watering", "grove"])
		}
		ForestGroveKind::TemperateLowerMassives | ForestGroveKind::TemperateMassives => {
			KindVocab::new(&["temperate", "broadleaf"], &["grove", "stand", "woods", "forest"])
		}
		ForestGroveKind::TradeWinds => {
			KindVocab::new(&["windward", "trade"], &["grove", "stand", "woods", "canopy"])
		}
		ForestGroveKind::TropicalThicket => {
			KindVocab::new(&["tropical", "dense"], &["thicket", "tangle", "brake", "scrub"])
		}
		ForestGroveKind::Vineyard => {
			KindVocab::new(&["tended", "climbing"], &["vineyard", "vines", "trellis", "plantings"])
		}
		ForestGroveKind::WanderingAcacia => {
			KindVocab::new(&["acacia", "wandering"], &["grove", "stand", "wood", "veld"])
		}
	}
}

fn urbanization_vocab(kind: UrbanizationKind) -> KindVocab {
	match kind {
		UrbanizationKind::None => KindVocab::heads_only(&[]),
		UrbanizationKind::MixedAgeCity | UrbanizationKind::ModernCity => {
			KindVocab::new(&["busy", "built"], &["city", "quarter", "district", "ward"])
		}
		UrbanizationKind::RuralLife => {
			KindVocab::new(&["rural", "quiet"], &["country", "hinterland", "parish", "district"])
		}
		UrbanizationKind::Townships => {
			KindVocab::new(&["settled"], &["town", "township", "borough", "village"])
		}
		UrbanizationKind::Frontier => {
			KindVocab::new(&["frontier", "far"], &["outpost", "march", "settlement", "camp"])
		}
		UrbanizationKind::Colony => {
			KindVocab::new(&["colonial", "planted"], &["colony", "settlement", "holding", "port"])
		}
	}
}

fn development_vocab(kind: UrbanDevelopmentKind) -> KindVocab {
	match kind {
		UrbanDevelopmentKind::Empty => KindVocab::heads_only(&[]),
		UrbanDevelopmentKind::LesHalles | UrbanDevelopmentKind::OldCityMarket => {
			KindVocab::new(&["busy", "open"], &["market", "bazaar", "exchange", "halls"])
		}
		UrbanDevelopmentKind::ShepherdsVillage => {
			KindVocab::new(&["shepherd", "pastoral"], &["village", "hamlet", "thorpe", "croft"])
		}
		UrbanDevelopmentKind::ShepherdsCommune => {
			KindVocab::new(&["shared", "pastoral"], &["commune", "holding", "stead", "croft"])
		}
		UrbanDevelopmentKind::RingFort => {
			KindVocab::new(&["ringed", "walled"], &["fort", "keep", "hold", "redoubt"])
		}
		UrbanDevelopmentKind::TempleComplex => {
			KindVocab::new(&["hallowed", "walled"], &["temple", "shrine", "sanctuary", "fane"])
		}
		UrbanDevelopmentKind::SingleHighrise => {
			KindVocab::new(&["tall", "single"], &["highrise", "tower", "block", "spire"])
		}
		UrbanDevelopmentKind::SuburbanHomes => {
			KindVocab::new(&["quiet", "low"], &["suburb", "homes", "row", "terrace"])
		}
		UrbanDevelopmentKind::WizardsTower => {
			KindVocab::new(&["wizard", "lone"], &["tower", "spire", "keep", "turret"])
		}
		UrbanDevelopmentKind::SkybridgeBazaar => {
			KindVocab::new(&["high", "spanning"], &["bazaar", "bridge", "market", "crossing"])
		}
	}
}

fn place_vocab(label: DiscoverablePlaceLabel) -> KindVocab {
	match label {
		DiscoverablePlaceLabel::Stall => {
			KindVocab::heads_only(&["stall", "booth", "stand", "kiosk"])
		}
		DiscoverablePlaceLabel::Lounge => {
			KindVocab::heads_only(&["lounge", "parlor", "salon", "sitting"])
		}
		DiscoverablePlaceLabel::Bedroom => {
			KindVocab::heads_only(&["bedroom", "chamber", "sleeping", "room"])
		}
		DiscoverablePlaceLabel::Sanctum => {
			KindVocab::heads_only(&["sanctum", "shrine", "oratory", "chapel"])
		}
		DiscoverablePlaceLabel::Storey => {
			KindVocab::heads_only(&["storey", "floor", "level", "landing"])
		}
		DiscoverablePlaceLabel::House => {
			KindVocab::heads_only(&["house", "home", "dwelling", "lodge"])
		}
		DiscoverablePlaceLabel::Hut => KindVocab::heads_only(&["hut", "shack", "cabin", "shelter"]),
		DiscoverablePlaceLabel::Market => {
			KindVocab::heads_only(&["market", "bazaar", "exchange", "mart"])
		}
		DiscoverablePlaceLabel::Highrise => {
			KindVocab::heads_only(&["highrise", "tower", "block", "spire"])
		}
		DiscoverablePlaceLabel::Skybridge => {
			KindVocab::heads_only(&["skybridge", "bridge", "crossing", "span"])
		}
		DiscoverablePlaceLabel::Tower => {
			KindVocab::heads_only(&["tower", "spire", "keep", "turret"])
		}
		DiscoverablePlaceLabel::Room => KindVocab::heads_only(&["room", "chamber", "cell", "hall"]),
	}
}

/// Seeded color adjectives so nearby groves and POIs of the same kind differ.
pub const PLACE_COLORS: &[&str] = &[
	"red", "ochre", "amber", "gold", "olive", "green", "teal", "blue", "violet", "rose", "ivory",
	"umber", "slate", "copper", "silver", "rust", "crimson", "saffron", "jade", "indigo", "pearl",
	"bronze",
];

/// One color from [`PLACE_COLORS`], stable for `seed`.
pub fn color_term(seed: u64) -> String {
	PLACE_COLORS[(mix(seed ^ 0xC010_A11A) as usize) % PLACE_COLORS.len()].to_owned()
}

/// Prepend a seeded color, keeping the kind terms after it.
pub fn with_color_name(mut english: Vec<String>, seed: u64) -> Vec<String> {
	let color = color_term(seed);
	english.retain(|word| word != &color);
	english.insert(0, color);
	english
}

fn flatten(vocab: KindVocab) -> Vec<String> {
	let mut out = owned(vocab.modifiers);
	out.extend(owned(vocab.heads));
	out
}

fn merge_vocabs(vocabs: impl IntoIterator<Item = KindVocab>) -> (Vec<String>, Vec<String>) {
	let mut modifiers = Vec::new();
	let mut heads = Vec::new();
	for vocab in vocabs {
		for word in vocab.modifiers {
			push_unique(&mut modifiers, (*word).to_owned());
		}
		for word in vocab.heads {
			push_unique(&mut heads, (*word).to_owned());
		}
	}
	(modifiers, heads)
}

fn unique_keep_order(words: &[&str]) -> Vec<String> {
	let mut out = Vec::new();
	for word in words {
		push_unique(&mut out, word.trim().to_ascii_lowercase());
	}
	out
}

fn push_unique(out: &mut Vec<String>, word: String) {
	if word.is_empty() || out.iter().any(|seen| seen == &word) {
		return;
	}
	out.push(word);
}

fn pick_word(words: &[String], seed: u64) -> String {
	words[(mix(seed) as usize) % words.len()].clone()
}

fn owned(words: &[&str]) -> Vec<String> {
	words.iter().map(|word| (*word).to_owned()).collect()
}
