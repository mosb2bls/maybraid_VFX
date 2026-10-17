//! WordNet-backed [`ConceptUniverse`].
//!
//! Reads the full Princeton `dict` via [`wordnet_db`]. WordNet remains
//! external semantic context: languages import only the neighborhood they need.
//!
//! WordNet 3.1 data is included under [`WORDNET_LICENSE`]. Princeton University
//! does not endorse this project. Citation: Princeton University, *About
//! WordNet*, WordNet, Princeton University, 2010.
//! <https://wordnet.princeton.edu/>

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use crate::concept::{
	Concept, ConceptId, ConceptRelation, ConceptUniverse, EnglishSenseLookup, NeighborhoodRequest,
	OverlayTag, Pos, RelationKind,
};
use crate::error::LanguageError;

mod dict;
mod morphy;

pub use dict::{bundled_dict_dir, WordNetDict};

/// Princeton WordNet 3.1 license text shipped with the asset dictionary.
pub const WORDNET_LICENSE: &str = include_str!("../../../assets/language/wordnet/LICENSE-WORDNET");

/// WordNet citation suitable for project attribution.
pub const WORDNET_CITATION: &str =
	"Princeton University, About WordNet, WordNet, Princeton University, 2010. https://wordnet.princeton.edu/";

/// Overlay tag for the copular classification predicate.
pub const CLASSIFIED_AS_TAG: u32 = 0xC1A5_51F1;
/// Canonical concept universe backed by WordNet synsets plus a thin overlay.
pub struct WordNetConceptUniverse {
	dict: WordNetDict,
	proper_names: HashMap<String, ConceptId>,
	overlay_glosses: HashMap<ConceptId, Vec<String>>,
	overlay_edges: HashMap<ConceptId, Vec<ConceptRelation>>,
}

impl WordNetConceptUniverse {
	pub fn from_dict(dict: WordNetDict) -> Self {
		let mut universe = Self {
			dict,
			proper_names: HashMap::new(),
			overlay_glosses: HashMap::new(),
			overlay_edges: HashMap::new(),
		};
		universe.install_classified_as();
		universe
	}

	pub fn from_dir(path: impl AsRef<Path>) -> Result<Self, LanguageError> {
		Ok(Self::from_dict(WordNetDict::from_dir(path)?))
	}

	pub fn bundled() -> Result<Self, LanguageError> {
		Self::from_dir(bundled_dict_dir())
	}

	pub fn with_proper_names<I, S>(mut self, names: I) -> Self
	where
		I: IntoIterator<Item = S>,
		S: AsRef<str>,
	{
		let person = self.dict.sense("person", Pos::Noun, 0);
		for name in names {
			let key = normalize_lemma(name.as_ref());
			if key.is_empty() {
				continue;
			}
			let id = OverlayTag::from_key(&key).concept_id();
			self.proper_names.insert(key.clone(), id);
			self.overlay_glosses.insert(id, vec![key]);
			if let Some(person) = person {
				self.overlay_edges.entry(id).or_default().push(ConceptRelation {
					source: id,
					target: person,
					kind: RelationKind::Associated,
					weight: 0.5,
				});
			}
		}
		self
	}

	/// Register surface labels (numerals, leftover names) without a person edge.
	pub fn with_labels<I, S>(mut self, names: I) -> Self
	where
		I: IntoIterator<Item = S>,
		S: AsRef<str>,
	{
		for name in names {
			let key = normalize_lemma(name.as_ref());
			if key.is_empty() {
				continue;
			}
			let id = OverlayTag::from_key(&key).concept_id();
			self.proper_names.entry(key.clone()).or_insert(id);
			self.overlay_glosses.entry(id).or_insert_with(|| vec![key]);
		}
		self
	}

	pub fn with_document_overlays(self, document: &crate::parse::DependencyDocument) -> Self {
		self.with_proper_names(document.proper_nouns()).with_labels(document.numerals())
	}

	fn install_classified_as(&mut self) {
		let id = ConceptId::overlay(CLASSIFIED_AS_TAG);
		self.overlay_glosses
			.insert(id, vec!["be".to_owned(), "classified_as".to_owned()]);
		if let Some(be) = self.dict.sense("be", Pos::Verb, 0) {
			self.overlay_edges.entry(id).or_default().push(ConceptRelation {
				source: id,
				target: be,
				kind: RelationKind::Associated,
				weight: 0.8,
			});
		}
	}

	pub fn classified_as(&self) -> ConceptId {
		ConceptId::overlay(CLASSIFIED_AS_TAG)
	}

	pub fn dict(&self) -> &WordNetDict {
		&self.dict
	}

	fn overlay_concept(&self, id: ConceptId) -> Option<Concept> {
		self.overlay_glosses
			.get(&id)
			.map(|glosses| Concept { id, english_glosses: glosses.clone() })
	}
}

impl ConceptUniverse for WordNetConceptUniverse {
	fn concept(&self, id: ConceptId) -> Option<Concept> {
		if let Some(overlay) = self.overlay_concept(id) {
			return Some(overlay);
		}
		self.dict.concept(id)
	}

	fn resolve_english(&self, term: &str) -> Vec<ConceptId> {
		let lemma = normalize_lemma(term);
		let mut ids = Vec::new();
		if let Some(proper) = self.proper_names.get(&lemma) {
			ids.push(*proper);
		}
		if lemma == "be" || lemma == "classified_as" {
			ids.push(self.classified_as());
		}
		ids.extend(self.dict.resolve_form(&lemma));
		ids
	}

	fn neighborhood(
		&self,
		concept: ConceptId,
		request: NeighborhoodRequest,
	) -> Vec<ConceptRelation> {
		let mut relations = Vec::new();
		let mut seen = HashSet::new();
		let mut queue = VecDeque::new();
		queue.push_back((concept, 0_u8));
		seen.insert(concept);

		while let Some((current, depth)) = queue.pop_front() {
			if relations.len() >= request.max_neighbors {
				break;
			}
			if depth >= request.max_depth {
				continue;
			}

			let mut outgoing = Vec::new();
			if let Some(overlay) = self.overlay_edges.get(&current) {
				outgoing.extend(overlay.iter().copied());
			}
			outgoing.extend(self.dict.relations(current));

			// Same-lemma synsets are associated, not identical concepts.
			if let Some(concept_rec) = self.concept(current) {
				for gloss in &concept_rec.english_glosses {
					for mate in self.dict.resolve_lemma(gloss) {
						if mate != current {
							outgoing.push(ConceptRelation {
								source: current,
								target: mate,
								kind: RelationKind::Associated,
								weight: 0.45,
							});
						}
					}
				}
			}

			outgoing.sort_by(|a, b| {
				b.weight.partial_cmp(&a.weight).unwrap_or(std::cmp::Ordering::Equal)
			});

			for relation in outgoing {
				if !request.allows(relation.kind) {
					continue;
				}
				if relations.len() >= request.max_neighbors {
					break;
				}
				if seen.insert(relation.target) {
					relations.push(relation);
					queue.push_back((relation.target, depth + 1));
				}
			}
		}

		relations
	}
}

impl EnglishSenseLookup for WordNetConceptUniverse {
	fn noun_sense(&self, lemma: &str, sense: usize) -> Result<ConceptId, LanguageError> {
		self.dict.sense(lemma, Pos::Noun, sense).ok_or_else(|| {
			if self.dict.has_lemma(lemma, Pos::Noun) {
				LanguageError::MissingSense {
					lemma: lemma.to_owned(),
					pos: Pos::Noun.to_string(),
					sense,
				}
			} else {
				LanguageError::MissingLemma { lemma: lemma.to_owned(), pos: Pos::Noun.to_string() }
			}
		})
	}

	fn verb_sense(&self, lemma: &str, sense: usize) -> Result<ConceptId, LanguageError> {
		self.dict.sense(lemma, Pos::Verb, sense).ok_or_else(|| {
			if self.dict.has_lemma(lemma, Pos::Verb) {
				LanguageError::MissingSense {
					lemma: lemma.to_owned(),
					pos: Pos::Verb.to_string(),
					sense,
				}
			} else {
				LanguageError::MissingLemma { lemma: lemma.to_owned(), pos: Pos::Verb.to_string() }
			}
		})
	}

	fn proper_name(&self, name: &str) -> Option<ConceptId> {
		self.proper_names.get(&normalize_lemma(name)).copied()
	}
}

pub fn normalize_lemma(term: &str) -> String {
	term.trim().to_ascii_lowercase().replace(' ', "_")
}
