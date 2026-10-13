//! Translate a subset of English kind terms in a tile language.

use maybraid_language_core::lexicalizer::mix;
use maybraid_language_core::{
	CompositionalLexicalizer, ConceptId, ConceptUniverse, InMemoryLexicalGraph, Lexicalizer,
	ModifierPlacement, Profile, RootHeavyLexicalizer,
};

use crate::bundle::{LanguageBundle, LexiconFamily};
use crate::catalog::KindConceptUniverse;

/// Assigned place name. Overlay concept is the proper-name handle.
///
/// Realization is lexicalization plus optional modifier-after-noun ordering.
/// Composite grammar (word order, agreement, particles) is not exercised yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaceName {
	pub surface: String,
	pub english: Vec<String>,
	pub overlay: ConceptId,
	pub language_seed: u64,
}

impl PlaceName {
	pub fn translate(bundle: &LanguageBundle, english: &[String], pick_seed: u64) -> Self {
		let chosen = pick_terms(english, pick_seed);
		let mut catalog = KindConceptUniverse::new();
		catalog.intern_all(chosen.iter());
		let mut graph = InMemoryLexicalGraph::default();
		let profile = Profile::neutral();
		let mut forms = Vec::new();
		for word in &chosen {
			let Some(concept) = catalog.resolve_english(word).into_iter().next() else {
				continue;
			};
			let term = lexicalize(bundle, concept, &catalog, &mut graph, &profile);
			forms.push(term);
		}
		if matches!(bundle.grammar.modifiers, ModifierPlacement::AfterNoun) && forms.len() > 1 {
			if let Some(head) = forms.pop() {
				forms.insert(0, head);
			}
		}
		let surface = forms.join(" ");
		let overlay = ConceptId::overlay(mix(bundle.seed ^ stable_surface(&surface)) as u32);
		Self { surface, english: chosen, overlay, language_seed: bundle.seed }
	}

	/// Translate every term in arrival order. Used when a color + kind must both survive.
	pub fn translate_all(bundle: &LanguageBundle, english: &[String]) -> Self {
		let chosen = unique_ordered(english);
		let mut catalog = KindConceptUniverse::new();
		catalog.intern_all(chosen.iter());
		let mut graph = InMemoryLexicalGraph::default();
		let profile = Profile::neutral();
		let mut forms = Vec::new();
		for word in &chosen {
			let Some(concept) = catalog.resolve_english(word).into_iter().next() else {
				continue;
			};
			forms.push(lexicalize(bundle, concept, &catalog, &mut graph, &profile));
		}
		if matches!(bundle.grammar.modifiers, ModifierPlacement::AfterNoun) && forms.len() > 1 {
			if let Some(head) = forms.pop() {
				forms.insert(0, head);
			}
		}
		let surface = forms.join(" ");
		let overlay = ConceptId::overlay(mix(bundle.seed ^ stable_surface(&surface)) as u32);
		Self { surface, english: chosen, overlay, language_seed: bundle.seed }
	}
}

fn unique_ordered(english: &[String]) -> Vec<String> {
	let mut unique = Vec::new();
	for word in english {
		let normalized = word.trim().to_ascii_lowercase();
		if normalized.is_empty() || unique.iter().any(|seen: &String| seen == &normalized) {
			continue;
		}
		unique.push(normalized);
	}
	unique
}

fn lexicalize(
	bundle: &LanguageBundle,
	concept: ConceptId,
	universe: &impl ConceptUniverse,
	graph: &mut InMemoryLexicalGraph,
	profile: &Profile,
) -> String {
	let realization = match bundle.lexicon {
		LexiconFamily::Compositional => {
			CompositionalLexicalizer::new(bundle.seed).lexicalize(concept, universe, graph, profile)
		}
		LexiconFamily::RootHeavy => {
			RootHeavyLexicalizer::new(bundle.seed).lexicalize(concept, universe, graph, profile)
		}
	};
	realization.term.ipa.0
}

/// Canonicalize, sort, and deduplicate before the seeded shuffle.
pub fn pick_terms(english: &[String], pick_seed: u64) -> Vec<String> {
	let mut unique = canonicalize_terms(english);
	if unique.is_empty() {
		return unique;
	}
	let take = (1 + (mix(pick_seed) as usize % 3.min(unique.len()))).min(unique.len());
	let mut rng = mix(pick_seed ^ 0xC0FF);
	for i in 0..unique.len() {
		rng = mix(rng);
		let j = (rng as usize) % unique.len();
		unique.swap(i, j);
	}
	unique.truncate(take);
	unique
}

/// Sorted unique lowercase terms. Input arrival order does not survive.
pub fn canonicalize_terms(english: &[String]) -> Vec<String> {
	let mut unique = Vec::new();
	for word in english {
		let normalized = word.trim().to_ascii_lowercase();
		if normalized.is_empty() || unique.iter().any(|seen: &String| seen == &normalized) {
			continue;
		}
		unique.push(normalized);
	}
	unique.sort_unstable();
	unique
}

fn stable_surface(surface: &str) -> u64 {
	let mut h = 0x811c_9dc5_u64;
	for byte in surface.as_bytes() {
		h = mix(h ^ u64::from(*byte));
	}
	h
}
