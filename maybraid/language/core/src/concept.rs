//! Canonical concepts, distinct from any language's lexical forms.

use std::fmt;

/// Stable identifier for a concept in a [`ConceptUniverse`].
///
/// WordNet synsets occupy namespace `0`. Overlay concepts (proper names and
/// other language-external handles) occupy namespace `1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConceptId(u64);

impl ConceptId {
	const POS_SHIFT: u64 = 32;
	const NS_SHIFT: u64 = 40;

	/// WordNet synset identified by byte offset and part of speech.
	pub const fn wordnet(offset: u32, pos: Pos) -> Self {
		Self((offset as u64) | ((pos as u64) << Self::POS_SHIFT))
	}

	/// Language-external concept (proper names, classification predicates).
	pub const fn overlay(tag: u32) -> Self {
		Self((tag as u64) | (1_u64 << Self::NS_SHIFT))
	}

	pub const fn as_u64(self) -> u64 {
		self.0
	}

	pub const fn namespace(self) -> ConceptNamespace {
		if (self.0 >> Self::NS_SHIFT) & 0xFF == 1 {
			ConceptNamespace::Overlay
		} else {
			ConceptNamespace::WordNet
		}
	}

	pub const fn offset(self) -> u32 {
		self.0 as u32
	}

	pub const fn pos(self) -> Option<Pos> {
		if matches!(self.namespace(), ConceptNamespace::WordNet) {
			Pos::from_code(((self.0 >> Self::POS_SHIFT) & 0xFF) as u8)
		} else {
			None
		}
	}
}

/// Low 24 bits of FNV-1a over UTF-8 bytes, used to mint [`ConceptId::overlay`] tags.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OverlayTag(u32);

impl OverlayTag {
	const FNV_OFFSET: u32 = 2_166_136_261;
	const FNV_PRIME: u32 = 16_777_619;
	pub const MASK: u32 = 0x00FF_FFFF;

	pub fn from_key(key: &str) -> Self {
		let mut hash = Self::FNV_OFFSET;
		for byte in key.as_bytes() {
			hash ^= u32::from(*byte);
			hash = hash.wrapping_mul(Self::FNV_PRIME);
		}
		Self(hash & Self::MASK)
	}

	pub const fn as_u32(self) -> u32 {
		self.0
	}

	pub const fn concept_id(self) -> ConceptId {
		ConceptId::overlay(self.0)
	}
}

impl fmt::Display for ConceptId {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self.namespace() {
			ConceptNamespace::WordNet => {
				let pos = self.pos().unwrap_or(Pos::Noun);
				write!(f, "{}:{:08}", pos.letter(), self.offset())
			}
			ConceptNamespace::Overlay => write!(f, "overlay:{:08x}", self.offset()),
		}
	}
}

/// Which catalog minted a [`ConceptId`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ConceptNamespace {
	WordNet,
	Overlay,
}

/// WordNet part of speech.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Pos {
	Noun = 1,
	Verb = 2,
	Adjective = 3,
	AdjectiveSatellite = 4,
	Adverb = 5,
}

impl Pos {
	pub const fn letter(self) -> char {
		match self {
			Self::Noun => 'n',
			Self::Verb => 'v',
			Self::Adjective => 'a',
			Self::AdjectiveSatellite => 's',
			Self::Adverb => 'r',
		}
	}

	pub const fn from_letter(letter: char) -> Option<Self> {
		match letter {
			'n' => Some(Self::Noun),
			'v' => Some(Self::Verb),
			'a' => Some(Self::Adjective),
			's' => Some(Self::AdjectiveSatellite),
			'r' => Some(Self::Adverb),
			_ => None,
		}
	}

	pub const fn from_code(code: u8) -> Option<Self> {
		match code {
			1 => Some(Self::Noun),
			2 => Some(Self::Verb),
			3 => Some(Self::Adjective),
			4 => Some(Self::AdjectiveSatellite),
			5 => Some(Self::Adverb),
			_ => None,
		}
	}

	pub const fn dict_stem(self) -> &'static str {
		match self {
			Self::Noun => "noun",
			Self::Verb => "verb",
			Self::Adjective | Self::AdjectiveSatellite => "adj",
			Self::Adverb => "adv",
		}
	}
}

impl fmt::Display for Pos {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.dict_stem())
	}
}

/// Immutable description of a concept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Concept {
	pub id: ConceptId,
	pub english_glosses: Vec<String>,
}

impl Concept {
	pub fn primary_gloss(&self) -> &str {
		self.english_glosses.first().map(String::as_str).unwrap_or("?")
	}
}

/// Directed typed relation between concepts in the canonical universe.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConceptRelation {
	pub source: ConceptId,
	pub target: ConceptId,
	pub kind: RelationKind,
	pub weight: f32,
}

/// Semantic neighborhood relation, independent of any language.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RelationKind {
	Synonym,
	Hypernym,
	Hyponym,
	PartOf,
	Antonym,
	Associated,
}

impl RelationKind {
	pub fn default_weight(self) -> f32 {
		match self {
			Self::Synonym => 1.0,
			Self::Hypernym => 0.75,
			Self::Hyponym => 0.65,
			Self::PartOf => 0.55,
			Self::Antonym => 0.4,
			Self::Associated => 0.35,
		}
	}
}

/// Bounds a [`ConceptUniverse::neighborhood`] query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NeighborhoodRequest {
	pub max_neighbors: usize,
	pub max_depth: u8,
	pub kinds: Option<Vec<RelationKind>>,
}

impl NeighborhoodRequest {
	pub fn bounded(max_neighbors: usize, max_depth: u8) -> Self {
		Self { max_neighbors, max_depth, kinds: None }
	}

	pub fn allows(&self, kind: RelationKind) -> bool {
		self.kinds.as_ref().map(|kinds| kinds.contains(&kind)).unwrap_or(true)
	}
}

impl Default for NeighborhoodRequest {
	fn default() -> Self {
		Self::bounded(8, 1)
	}
}

/// Immutable English-facing concept catalog.
pub trait ConceptUniverse {
	fn concept(&self, id: ConceptId) -> Option<Concept>;

	fn resolve_english(&self, term: &str) -> Vec<ConceptId>;

	fn neighborhood(
		&self,
		concept: ConceptId,
		request: NeighborhoodRequest,
	) -> Vec<ConceptRelation>;
}

/// Sense-indexed English lookup used by POC utterance builders.
pub trait EnglishSenseLookup {
	fn noun_sense(
		&self,
		lemma: &str,
		sense: usize,
	) -> Result<ConceptId, crate::error::LanguageError>;

	fn verb_sense(
		&self,
		lemma: &str,
		sense: usize,
	) -> Result<ConceptId, crate::error::LanguageError>;

	fn proper_name(&self, name: &str) -> Option<ConceptId>;
}

#[cfg(test)]
mod overlay_tag_tests {
	use super::OverlayTag;

	#[test]
	fn from_key_is_stable_and_masked() {
		let first = OverlayTag::from_key("Mary");
		let second = OverlayTag::from_key("Mary");
		assert_eq!(first, second);
		assert_eq!(first.as_u32() & OverlayTag::MASK, first.as_u32());
	}

	#[test]
	fn from_key_differs_across_keys() {
		assert_ne!(OverlayTag::from_key("Mary"), OverlayTag::from_key("John"));
	}
}
