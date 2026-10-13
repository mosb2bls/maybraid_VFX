//! Adapter over [`wordnet_db`] for the full Princeton `dict` directory.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use wordnet_db::{LoadMode, WordNet};
use wordnet_types::{Pos as WnPos, SynsetId};

use crate::concept::{Concept, ConceptId, ConceptRelation, Pos, RelationKind};
use crate::error::LanguageError;
use crate::wordnet::morphy::Morphy;
use crate::wordnet::normalize_lemma;

/// Directory containing the vendored WordNet 3.1 `data.*` / `index.*` files.
pub fn bundled_dict_dir() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/language/wordnet/dict")
}

/// Full WordNet dictionary via mmap. Languages still import neighborhoods lazily.
pub struct WordNetDict {
	inner: WordNet,
	morphy: Morphy,
}

impl WordNetDict {
	pub fn from_dir(path: impl AsRef<Path>) -> Result<Self, LanguageError> {
		let path = path.as_ref();
		let inner = WordNet::load_with_mode(path, LoadMode::Mmap).map_err(|source| {
			LanguageError::WordNet { path: path.display().to_string(), detail: source.to_string() }
		})?;
		Ok(Self { inner, morphy: Morphy::from_dir(path)? })
	}

	pub fn concept(&self, id: ConceptId) -> Option<Concept> {
		let synset = self.inner.get_synset(to_synset_id(id)?)?;
		Some(Concept {
			id,
			english_glosses: synset.words.iter().map(|lemma| normalize_lemma(lemma.text)).collect(),
		})
	}

	pub fn resolve_lemma(&self, lemma: &str) -> Vec<ConceptId> {
		self.lookup_form(&normalize_lemma(lemma))
	}

	/// Citation-form lookup plus Morphy (`struck` → `strike`, `children` → `child`).
	pub fn resolve_form(&self, form: &str) -> Vec<ConceptId> {
		let form = normalize_lemma(form);
		let mut ids = Vec::new();
		let mut seen = HashSet::new();
		for pos in [Pos::Noun, Pos::Verb, Pos::Adjective, Pos::Adverb] {
			for candidate in self.morphy.candidates(&form, pos) {
				for id in self.lookup_pos(&candidate, pos) {
					if seen.insert(id) {
						ids.push(id);
					}
				}
			}
		}
		ids
	}

	fn lookup_form(&self, lemma: &str) -> Vec<ConceptId> {
		let mut ids = Vec::new();
		for pos in [WnPos::Noun, WnPos::Verb, WnPos::Adj, WnPos::Adv] {
			for synset_id in self.inner.synsets_for_lemma(pos, lemma) {
				ids.push(from_synset_id(*synset_id));
			}
		}
		ids
	}

	fn lookup_pos(&self, lemma: &str, pos: Pos) -> Vec<ConceptId> {
		let Some(wn_pos) = to_wn_pos(pos) else {
			return Vec::new();
		};
		self.inner
			.synsets_for_lemma(wn_pos, lemma)
			.iter()
			.copied()
			.map(from_synset_id)
			.collect()
	}

	pub fn sense(&self, lemma: &str, pos: Pos, sense: usize) -> Option<ConceptId> {
		self.inner
			.synsets_for_lemma(to_wn_pos(pos)?, &normalize_lemma(lemma))
			.get(sense)
			.copied()
			.map(from_synset_id)
	}

	pub fn has_lemma(&self, lemma: &str, pos: Pos) -> bool {
		to_wn_pos(pos)
			.map(|wn_pos| self.inner.lemma_exists(wn_pos, &normalize_lemma(lemma)))
			.unwrap_or(false)
	}

	pub fn relations(&self, id: ConceptId) -> Vec<ConceptRelation> {
		let Some(synset_id) = to_synset_id(id) else {
			return Vec::new();
		};
		let Some(synset) = self.inner.get_synset(synset_id) else {
			return Vec::new();
		};
		let source = from_synset_id(synset.id);
		synset
			.pointers
			.iter()
			.filter_map(|pointer| {
				let (kind, weight) = map_pointer(pointer.symbol)?;
				Some(ConceptRelation {
					source,
					target: from_synset_id(pointer.target),
					kind,
					weight,
				})
			})
			.collect()
	}
}

fn to_wn_pos(pos: Pos) -> Option<WnPos> {
	match pos {
		Pos::Noun => Some(WnPos::Noun),
		Pos::Verb => Some(WnPos::Verb),
		Pos::Adjective | Pos::AdjectiveSatellite => Some(WnPos::Adj),
		Pos::Adverb => Some(WnPos::Adv),
	}
}

fn from_wn_pos(pos: WnPos) -> Pos {
	match pos {
		WnPos::Noun => Pos::Noun,
		WnPos::Verb => Pos::Verb,
		WnPos::Adj => Pos::Adjective,
		WnPos::Adv => Pos::Adverb,
	}
}

fn to_synset_id(id: ConceptId) -> Option<SynsetId> {
	Some(SynsetId { pos: to_wn_pos(id.pos()?)?, offset: id.offset() })
}

fn from_synset_id(id: SynsetId) -> ConceptId {
	ConceptId::wordnet(id.offset, from_wn_pos(id.pos))
}

fn map_pointer(symbol: &str) -> Option<(RelationKind, f32)> {
	match symbol {
		"!" => Some((RelationKind::Antonym, 0.4)),
		"@" | "@i" => Some((RelationKind::Hypernym, 0.75)),
		"~" => Some((RelationKind::Hyponym, 0.6)),
		"~i" => Some((RelationKind::Hyponym, 0.2)),
		"#m" | "#s" | "#p" | "%m" | "%s" | "%p" => Some((RelationKind::PartOf, 0.55)),
		"+" => Some((RelationKind::Associated, 0.4)),
		"&" => Some((RelationKind::Synonym, 0.85)),
		"$" | "*" | ">" | "=" | "^" | ";c" | ";r" | ";u" | "-c" | "-r" | "-u" => {
			Some((RelationKind::Associated, 0.3))
		}
		_ => None,
	}
}
