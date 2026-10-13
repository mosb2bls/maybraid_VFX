use std::cell::RefCell;

use anyhow::{anyhow, Context};

use crate::concept::{
	Concept, ConceptId, ConceptRelation, ConceptUniverse, EnglishSenseLookup, NeighborhoodRequest,
};
use crate::grammar::SurfaceGrammar;
use crate::graph::{InMemoryLexicalGraph, LexicalContextGraph};
use crate::lexicalizer::{CompositionalLexicalizer, Lexicalizer, RootHeavyLexicalizer};
use crate::marshall::{ConceptMarshaller, DefaultMarshaller};
use crate::output::LexicalOutput;
use crate::parse::EnglishDependencyParser;
use crate::poc::{poc_universe, PocLexicon};
use crate::profile::Profile;
use crate::semantic::{EnglishSemanticMarshaller, SemanticMarshaller};
use crate::utterance::{Polarity, SemanticValue, Utterance};

struct CountingUniverse<U> {
	inner: U,
	neighborhood_queries: RefCell<Vec<ConceptId>>,
}

impl<U: ConceptUniverse> ConceptUniverse for CountingUniverse<U> {
	fn concept(&self, id: ConceptId) -> Option<Concept> {
		self.inner.concept(id)
	}

	fn resolve_english(&self, term: &str) -> Vec<ConceptId> {
		self.inner.resolve_english(term)
	}

	fn neighborhood(
		&self,
		concept: ConceptId,
		request: NeighborhoodRequest,
	) -> Vec<ConceptRelation> {
		self.neighborhood_queries.borrow_mut().push(concept);
		self.inner.neighborhood(concept, request)
	}
}

fn lex() -> anyhow::Result<(crate::WordNetConceptUniverse, PocLexicon)> {
	let universe = poc_universe()?;
	let lex = PocLexicon::from_universe(&universe)?;
	Ok((universe, lex))
}

fn render_pair(
	utterance: Utterance,
	universe: &impl ConceptUniverse,
) -> (LexicalOutput, InMemoryLexicalGraph, LexicalOutput, InMemoryLexicalGraph) {
	let profile = Profile::neutral();
	let mut graph_a = InMemoryLexicalGraph::new();
	let out_a = LexicalOutput::render(
		utterance.clone(),
		&CompositionalLexicalizer::default(),
		universe,
		&mut graph_a,
		&profile,
	);
	let mut graph_b = InMemoryLexicalGraph::new();
	let out_b = LexicalOutput::render(
		utterance,
		&RootHeavyLexicalizer::default(),
		universe,
		&mut graph_b,
		&profile,
	);
	(out_a, graph_a, out_b, graph_b)
}

#[test]
fn wordnet_resolves_lemmas_to_synsets_not_strings() -> anyhow::Result<()> {
	let universe = poc_universe()?;
	let gives = universe.resolve_english("give");
	assert!(
		gives.len() >= 2,
		"give should map to multiple synsets, not the English string, got {}",
		gives.len()
	);
	assert!(gives.iter().all(|id| universe.concept(*id).is_some()));
	let glosses: Vec<_> = gives
		.iter()
		.filter_map(|id| universe.concept(*id))
		.map(|c| c.primary_gloss().to_owned())
		.collect();
	assert!(glosses.iter().any(|g| g == "give"));
	Ok(())
}

#[test]
fn wordnet_reduces_inflected_verbs_to_citation_forms() -> anyhow::Result<()> {
	let universe = poc_universe()?;
	let strikes = universe.resolve_english("strikes");
	assert!(!strikes.is_empty(), "strikes should reduce to WordNet strike");
	let glosses: Vec<_> = strikes
		.iter()
		.filter_map(|id| universe.concept(*id))
		.map(|c| c.primary_gloss().to_owned())
		.collect();
	assert!(glosses.iter().any(|g| g == "strike"));
	assert!(universe
		.resolve_english("gives")
		.iter()
		.filter_map(|id| universe.concept(*id))
		.any(|concept| concept.primary_gloss() == "give"));
	assert!(universe
		.resolve_english("struck")
		.iter()
		.filter_map(|id| universe.concept(*id))
		.any(|concept| concept.primary_gloss() == "strike"));
	assert!(universe
		.resolve_english("children")
		.iter()
		.filter_map(|id| universe.concept(*id))
		.any(|concept| concept.primary_gloss() == "child"));
	assert!(universe
		.resolve_english("went")
		.iter()
		.filter_map(|id| universe.concept(*id))
		.any(|concept| concept.primary_gloss() == "go"));
	Ok(())
}

#[test]
fn wordnet_neighborhood_is_bounded() -> anyhow::Result<()> {
	let universe = poc_universe()?;
	let stream = universe.noun_sense("stream", 0)?;
	let neighborhood = universe.neighborhood(stream, NeighborhoodRequest::bounded(6, 1));
	assert!(!neighborhood.is_empty());
	assert!(neighborhood.len() <= 6);
	assert!(neighborhood.iter().all(|rel| rel.source == stream));
	Ok(())
}

#[test]
fn utterance_supports_coreference_and_relative_clauses() -> anyhow::Result<()> {
	let (universe, lex) = lex()?;
	let utterance = Utterance::john_gave_the_book_to_mary_the_witch(&lex);
	assert_eq!(utterance.roots.len(), 1);
	let give = utterance.roots[0];
	let give_clause = utterance.clauses.get(give).context("missing give")?;
	let mary = give_clause
		.arguments
		.iter()
		.find(|argument| argument.role == crate::SemanticRole::Recipient)
		.and_then(|argument| match argument.value {
			SemanticValue::Referent(id) => Some(id),
			SemanticValue::Clause(_) => None,
		})
		.context("missing mary")?;
	let mary_ref = utterance.referents.get(mary).context("mary referent")?;
	assert_eq!(mary_ref.relative_clauses.len(), 1);
	let classified = utterance
		.clauses
		.get(mary_ref.relative_clauses[0])
		.context("classified clause")?;
	assert_eq!(classified.predicate, lex.classified_as);
	let witch = classified
		.arguments
		.iter()
		.find(|argument| argument.role == crate::SemanticRole::Classification)
		.and_then(|argument| match argument.value {
			SemanticValue::Referent(id) => Some(id),
			SemanticValue::Clause(_) => None,
		})
		.context("missing witch")?;
	assert_eq!(utterance.referents.get(witch).context("witch")?.relative_clauses.len(), 1);

	let uses = DefaultMarshaller.concepts(&utterance);
	let mary_uses = uses.iter().filter(|item| item.concept == lex.mary).count();
	assert_eq!(mary_uses, 1, "coreferent Mary should marshall once as a referent");
	assert!(uses.iter().any(|item| item.concept == lex.witch));
	assert!(uses.iter().any(|item| item.concept == lex.helper));
	assert!(universe.concept(lex.give).is_some());
	Ok(())
}

#[test]
fn both_languages_render_the_simple_give_sentence() -> anyhow::Result<()> {
	let (universe, lex) = lex()?;
	let (out_a, _, out_b, _) = render_pair(Utterance::john_gave_the_book_to_mary(&lex), &universe);
	assert!(!out_a.lexicalizations.is_empty());
	assert_eq!(out_a.lexicalizations.len(), out_b.lexicalizations.len());
	for concept in [lex.john, lex.mary, lex.book, lex.give] {
		let a = out_a.realization_of(concept).context("language A missing concept")?;
		let b = out_b.realization_of(concept).context("language B missing concept")?;
		assert!(!a.term.ipa.as_str().is_empty());
		assert!(!b.term.ipa.as_str().is_empty());
	}
	Ok(())
}

#[test]
fn both_languages_render_the_witch_sentence() -> anyhow::Result<()> {
	let (universe, lex) = lex()?;
	let (out_a, graph_a, out_b, graph_b) =
		render_pair(Utterance::john_gave_the_book_to_mary_the_witch(&lex), &universe);
	assert!(out_a.realization_of(lex.witch).is_some());
	assert!(out_b.realization_of(lex.helper).is_some());
	let report = out_a.debug_report("LANGUAGE A", &universe, &graph_a);
	assert!(report.contains("realization:"), "{report}");
	assert!(out_b.debug_report("LANGUAGE B", &universe, &graph_b).contains("provenance:"));
	Ok(())
}

#[test]
fn stream_sentence_builds_different_lexical_families() -> anyhow::Result<()> {
	let (universe, lex) = lex()?;
	let (out_a, graph_a, out_b, graph_b) =
		render_pair(Utterance::stream_carried_water(&lex), &universe);
	let a_stream = out_a.realization_of(lex.stream).context("A stream")?;
	let b_stream = out_b.realization_of(lex.stream).context("B stream")?;
	assert_ne!(
		a_stream.term.ipa, b_stream.term.ipa,
		"languages should coin visibly different STREAM terms"
	);
	assert!(
		a_stream.provenance.len() >= 2,
		"compositional STREAM should recombine neighborhood material: {:?}",
		a_stream.provenance
	);
	assert!(
		a_stream.provenance.iter().any(|part| part.source_concept != lex.stream),
		"compositional STREAM should not be a lone coined root"
	);
	assert!(
		b_stream.provenance.iter().any(|part| part.source_concept == lex.stream)
			|| b_stream.term.ipa.as_str().len() >= a_stream.term.ipa.as_str().len(),
		"root-heavy STREAM should stay an independent root or occasional blend"
	);

	let a_water = out_a.realization_of(lex.water).context("A water")?;
	let a_report = out_a.debug_concept(lex.stream, &universe, &graph_a).context("debug A")?;
	assert!(a_report.contains("canonical neighborhood:"), "{a_report}");
	assert!(a_report.contains(&a_stream.term.ipa.to_string()), "{a_report}");
	assert!(!a_water.term.ipa.as_str().is_empty());

	let b_report = out_b.debug_concept(lex.stream, &universe, &graph_b).context("debug B")?;
	assert!(b_report.contains("realization:"), "{b_report}");
	Ok(())
}

#[test]
fn lexicalization_is_deterministic() -> anyhow::Result<()> {
	let (universe, lex) = lex()?;
	let utterance = Utterance::stream_carried_water(&lex);
	let (first, _, _, _) = render_pair(utterance.clone(), &universe);
	let (second, _, _, _) = render_pair(utterance, &universe);
	let a1 = first.realization_of(lex.stream).context("first")?;
	let a2 = second.realization_of(lex.stream).context("second")?;
	assert_eq!(a1.term, a2.term);
	assert_eq!(a1.provenance, a2.provenance);
	Ok(())
}

#[test]
fn established_concepts_do_not_requery_wordnet() -> anyhow::Result<()> {
	let (universe, lex) = lex()?;
	let counted =
		CountingUniverse { inner: universe, neighborhood_queries: RefCell::new(Vec::new()) };
	let profile = Profile::neutral();
	let lexicalizer = CompositionalLexicalizer::default();
	let mut graph = InMemoryLexicalGraph::new();
	lexicalizer.lexicalize(lex.stream, &counted, &mut graph, &profile);
	let first = counted.neighborhood_queries.borrow().clone();
	assert!(first.contains(&lex.stream), "first pass should query STREAM");
	counted.neighborhood_queries.borrow_mut().clear();
	lexicalizer.lexicalize(lex.stream, &counted, &mut graph, &profile);
	assert!(
		counted.neighborhood_queries.borrow().is_empty(),
		"established STREAM must not rebuild its WordNet neighborhood"
	);
	assert!(graph.neighborhood_imported(lex.stream));
	assert!(graph.contains_concept(lex.stream));
	Ok(())
}

#[test]
fn register_can_change_resolution_without_changing_semantics() -> anyhow::Result<()> {
	let (universe, lex) = lex()?;
	let lexicalizer = CompositionalLexicalizer::default();
	let mut familiar_graph = InMemoryLexicalGraph::new();
	let familiar =
		lexicalizer.lexicalize(lex.stream, &universe, &mut familiar_graph, &Profile::familiar());
	let mut formal_graph = InMemoryLexicalGraph::new();
	let formal =
		lexicalizer.lexicalize(lex.stream, &universe, &mut formal_graph, &Profile::formal());
	// Formal prefers compounding when donors exist; familiar may reuse a single donor.
	if formal.provenance.len() >= 2 {
		assert_ne!(formal.term, familiar.term);
	} else {
		return Err(anyhow!("formal register should still see neighborhood donors"));
	}
	Ok(())
}

#[test]
fn surface_grammar_formats_an_ipa_sentence() -> anyhow::Result<()> {
	let (universe, lex) = lex()?;
	let (out_a, _, out_b, _) = render_pair(Utterance::john_gave_the_book_to_mary(&lex), &universe);
	let a = out_a.realize(SurfaceGrammar::compositional());
	let b = out_b.realize(SurfaceGrammar::root_heavy());
	let john_a = out_a.realization_of(lex.john).context("john A")?.term.ipa.as_str();
	let give_a = out_a.realization_of(lex.give).context("give A")?.term.ipa.as_str();
	let book_a = out_a.realization_of(lex.book).context("book A")?.term.ipa.as_str();
	let mary_a = out_a.realization_of(lex.mary).context("mary A")?.term.ipa.as_str();
	let ga = SurfaceGrammar::compositional();
	assert_eq!(
		a.ipa().to_string(),
		format!("/{john_a} {give_a} {book_a} {} {mary_a}/", ga.particles.recipient)
	);
	let john_b = out_b.realization_of(lex.john).context("john B")?.term.ipa.as_str();
	let give_b = out_b.realization_of(lex.give).context("give B")?.term.ipa.as_str();
	let book_b = out_b.realization_of(lex.book).context("book B")?.term.ipa.as_str();
	let mary_b = out_b.realization_of(lex.mary).context("mary B")?.term.ipa.as_str();
	let gb = SurfaceGrammar::root_heavy();
	assert_eq!(
		b.ipa().to_string(),
		format!("/{john_b} {book_b} {} {mary_b} {give_b}/", gb.particles.recipient)
	);
	assert!(
		!a.ipa().to_string().is_ascii(),
		"language A sentence should use Unicode IPA: {}",
		a.ipa()
	);
	assert!(
		!b.ipa().to_string().is_ascii(),
		"language B sentence should use Unicode IPA: {}",
		b.ipa()
	);
	Ok(())
}

#[test]
fn surface_grammar_binds_relative_clauses_without_repeating_the_head() -> anyhow::Result<()> {
	let (universe, lex) = lex()?;
	let (out_a, _, _, _) =
		render_pair(Utterance::john_gave_the_book_to_mary_the_witch(&lex), &universe);
	let sentence = out_a.realize(SurfaceGrammar::compositional()).ipa().to_string();
	let mary = out_a.realization_of(lex.mary).context("mary")?.term.ipa.as_str();
	let witch = out_a.realization_of(lex.witch).context("witch")?.term.ipa.as_str();
	let helper = out_a.realization_of(lex.helper).context("helper")?.term.ipa.as_str();
	assert!(sentence.starts_with('/'), "{sentence}");
	assert!(sentence.ends_with('/'), "{sentence}");
	assert!(sentence.contains(" | "), "{sentence}");
	let plural = SurfaceGrammar::compositional().particles.plural;
	assert!(
		sentence.contains(&format!(" {plural}/")) || sentence.contains(&format!(" {plural} ")),
		"{sentence}"
	);
	assert_eq!(
		sentence.matches(mary).count(),
		1,
		"relative 'who' should bind Mary, not repeat her: {sentence}"
	);
	assert!(sentence.contains(witch), "{sentence}");
	assert!(sentence.contains(helper), "{sentence}");
	Ok(())
}

#[test]
fn surface_grammar_emits_a_negation_particle() -> anyhow::Result<()> {
	let (universe, lex) = lex()?;
	let affirmative = Utterance::john_gave_the_book_to_mary(&lex);
	let mut negative = affirmative.clone();
	let root = negative.roots[0];
	negative.clauses[root].polarity = Polarity::Negative;
	let (out_yes, _, _, _) = render_pair(affirmative, &universe);
	let (out_no, _, _, _) = render_pair(negative, &universe);
	let grammar = SurfaceGrammar::compositional();
	let yes = out_yes.realize(grammar).ipa().to_string();
	let no = out_no.realize(grammar).ipa().to_string();
	assert_ne!(yes, no, "negation must change the IPA string");
	assert!(
		no.contains(grammar.particles.negative),
		"negative clause should carry {negative}: {no}",
		negative = grammar.particles.negative
	);
	assert!(
		!yes.split_whitespace().any(|word| word == grammar.particles.negative),
		"affirmative clause should omit the particle: {yes}"
	);
	Ok(())
}

#[test]
fn live_translate_negation_changes_ipa() -> anyhow::Result<()> {
	let Ok(parser) = crate::udpipe::UdpipeEnglishParser::bundled() else {
		return Ok(());
	};
	let universe =
		poc_universe()?.with_proper_names(["John", "Mary", "Alice", "speaker", "listener"]);
	let marshaller = EnglishSemanticMarshaller::default();
	let grammar = SurfaceGrammar::compositional();
	let realize = |text: &str| -> anyhow::Result<String> {
		let document = parser.parse(text)?;
		let utterance = marshaller.marshal(&document, &universe)?;
		let (output, _, _, _) = render_pair(utterance, &universe);
		Ok(output.realize(grammar).ipa().to_string())
	};
	let negative = realize("I don't know.")?;
	let emphatic = realize("I do know.")?;
	let full = realize("I do not know.")?;
	assert_ne!(negative, emphatic, "don't vs do: {negative} vs {emphatic}");
	assert_eq!(negative, full, "n't and not should realize the same polarity");
	assert!(
		negative.contains(grammar.particles.negative),
		"expected {} in {negative}",
		grammar.particles.negative
	);
	assert!(
		!emphatic.split_whitespace().any(|word| word == grammar.particles.negative),
		"affirmative should omit the particle: {emphatic}"
	);
	Ok(())
}

#[test]
fn wordnet_license_is_present() {
	assert!(crate::WORDNET_LICENSE.contains("Princeton University"));
	assert!(crate::WORDNET_LICENSE.contains("WordNet 3.1"));
	assert!(crate::WORDNET_CITATION.contains("wordnet.princeton.edu"));
}
