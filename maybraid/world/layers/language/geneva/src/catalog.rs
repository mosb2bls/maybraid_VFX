//! Small [`ConceptUniverse`] for kind-vocabulary English. Not WordNet.

use std::collections::HashMap;

use maybraid_language_core::lexicalizer::mix;
use maybraid_language_core::{
	Concept, ConceptId, ConceptRelation, ConceptUniverse, NeighborhoodRequest,
};

/// Overlay concepts keyed by lowercase English atoms from kind labels.
#[derive(Clone, Debug, Default)]
pub struct KindConceptUniverse {
	by_word: HashMap<String, ConceptId>,
	concepts: HashMap<ConceptId, Concept>,
}

impl KindConceptUniverse {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn intern(&mut self, word: &str) -> ConceptId {
		let key = word.trim().to_ascii_lowercase();
		if let Some(&id) = self.by_word.get(&key) {
			return id;
		}
		let id = ConceptId::overlay(stable_tag(&key));
		self.by_word.insert(key.clone(), id);
		self.concepts.insert(id, Concept { id, english_glosses: vec![key] });
		id
	}

	pub fn intern_all<I>(&mut self, words: I)
	where
		I: IntoIterator,
		I::Item: AsRef<str>,
	{
		for word in words {
			self.intern(word.as_ref());
		}
	}
}

impl ConceptUniverse for KindConceptUniverse {
	fn concept(&self, id: ConceptId) -> Option<Concept> {
		self.concepts.get(&id).cloned()
	}

	fn resolve_english(&self, term: &str) -> Vec<ConceptId> {
		let key = term.trim().to_ascii_lowercase();
		self.by_word.get(&key).copied().into_iter().collect()
	}

	fn neighborhood(
		&self,
		_concept: ConceptId,
		_request: NeighborhoodRequest,
	) -> Vec<ConceptRelation> {
		Vec::new()
	}
}

fn stable_tag(word: &str) -> u32 {
	let mut h = 0x811c_9dc5_u64;
	for byte in word.as_bytes() {
		h = mix(h ^ u64::from(*byte));
	}
	h as u32
}
