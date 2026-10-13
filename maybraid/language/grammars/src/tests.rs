use anyhow::Context;
use maybraid_language_core::{
	poc_universe, Clause, CompositionalLexicalizer, ConceptUniverse, EnglishSenseLookup,
	FocusTarget, Grammar, InMemoryLexicalGraph, LexicalOutput, Modifier, PocLexicon, Polarity,
	Profile, Referent, SemanticNode, SemanticRole, SemanticValue, SurfaceConstituent,
	SurfaceGrammar, Tense, Utterance,
};

use crate::{
	agglutinative_sov, ergative_vso, fusional_svo, isolating_svo, particle_heavy_topic_prominent,
	separable_svo, serial_verb, Alignment, CopulaStrategy, WordOrder,
};

fn lex() -> anyhow::Result<(maybraid_language_core::WordNetConceptUniverse, PocLexicon)> {
	let universe = poc_universe()?.with_proper_names(["John", "Mary"]);
	let lex = PocLexicon::from_universe(&universe)?;
	Ok((universe, lex))
}

fn render(utterance: Utterance, universe: &impl ConceptUniverse) -> LexicalOutput {
	let mut graph = InMemoryLexicalGraph::new();
	LexicalOutput::render(
		utterance,
		&CompositionalLexicalizer::default(),
		universe,
		&mut graph,
		&Profile::neutral(),
	)
}

fn words(output: &LexicalOutput, grammar: impl Grammar) -> Vec<String> {
	output.realize(grammar).words
}

fn john_cut_meat_with_knife(
	universe: &impl EnglishSenseLookup,
	lex: &PocLexicon,
) -> anyhow::Result<Utterance> {
	let cut = universe.verb_sense("cut", 0)?;
	let meat = universe.noun_sense("meat", 0)?;
	let knife = universe.noun_sense("knife", 0)?;
	let mut utterance = Utterance::new();
	let john = utterance.add_referent(Referent::new(lex.john).proper());
	let meat = utterance.add_referent(Referent::new(meat));
	let knife = utterance.add_referent(Referent::new(knife));
	let clause = utterance.add_clause(
		Clause::new(cut)
			.past()
			.with_argument(SemanticRole::Agent, SemanticValue::Referent(john))
			.with_argument(SemanticRole::Theme, SemanticValue::Referent(meat))
			.with_argument(SemanticRole::Instrument, SemanticValue::Referent(knife)),
	);
	utterance.push_root(clause);
	Ok(utterance)
}

#[test]
fn same_utterance_yields_different_orders() -> anyhow::Result<()> {
	let (universe, lex) = lex()?;
	let output = render(Utterance::john_gave_the_book_to_mary(&lex), &universe);
	let svo = words(&output, isolating_svo());
	let sov = words(&output, agglutinative_sov());
	let vso = words(&output, ergative_vso());
	assert_ne!(svo, sov);
	assert_ne!(svo, vso);
	assert_ne!(sov, vso);
	Ok(())
}

#[test]
fn nominative_and_ergative_marking_differ() -> anyhow::Result<()> {
	let (universe, lex) = lex()?;
	let output = render(Utterance::john_gave_the_book_to_mary(&lex), &universe);
	let na = words(&output, isolating_svo());
	let ea = words(&output, ergative_vso());
	assert!(
		!na.iter().any(|word| word == "ek"),
		"nominative-accusative should not use the ergative particle: {na:?}"
	);
	assert!(ea.iter().any(|word| word == "ek"), "ergative alignment should mark the agent: {ea:?}");
	Ok(())
}

#[test]
fn modifiers_and_relatives_can_move() -> anyhow::Result<()> {
	let (universe, lex) = lex()?;
	let red = universe.noun_sense("red", 0)?;
	let mut utterance = Utterance::john_gave_the_book_to_mary_the_witch(&lex);
	let book = utterance
		.referents
		.iter()
		.find(|(_, referent)| referent.concept == lex.book)
		.map(|(id, _)| id)
		.context("book")?;
	if let Some(book) = utterance.referents.get_mut(book) {
		book.modifiers.push(Modifier { concept: red });
	}
	let output = render(utterance, &universe);
	let after = output.realize_surface(isolating_svo());
	let before = output.realize_surface(agglutinative_sov());
	let after_forms = after.forms();
	let before_forms = before.forms();
	assert_ne!(after_forms, before_forms);
	assert!(after_forms.contains(&"|"));
	assert!(before_forms.contains(&"|"));
	assert!(
		after_forms.contains(&"de"),
		"isolating grammar should insert a modifier linker: {after_forms:?}"
	);
	Ok(())
}

#[test]
fn particles_mark_tam_and_negation() -> anyhow::Result<()> {
	let (universe, lex) = lex()?;
	let mut utterance = Utterance::john_gave_the_book_to_mary(&lex);
	let root = utterance.roots[0];
	if let Some(clause) = utterance.clauses.get_mut(root) {
		clause.tense = Tense::Past;
		clause.polarity = Polarity::Negative;
	}
	let output = render(utterance, &universe);
	let realized = words(&output, isolating_svo());
	assert!(realized.iter().any(|word| word == "le"), "past particle: {realized:?}");
	assert!(realized.iter().any(|word| word == "bu"), "negation particle: {realized:?}");
	Ok(())
}

#[test]
fn separable_predicate_is_discontinuous() -> anyhow::Result<()> {
	let (universe, lex) = lex()?;
	let output = render(Utterance::john_gave_the_book_to_mary(&lex), &universe);
	let surface = output.realize_surface(separable_svo());
	let parts: Vec<_> = surface
		.clauses
		.iter()
		.flat_map(|clause| clause.constituents.iter())
		.filter_map(|item| match item {
			SurfaceConstituent::Lexical { part, form, .. } => Some((*part, form.as_str())),
			_ => None,
		})
		.collect();
	let stem = parts
		.iter()
		.position(|(part, _)| *part == maybraid_language_core::LexicalPart::Stem)
		.context("missing stem")?;
	let particle = parts
		.iter()
		.position(|(part, form)| {
			*part == maybraid_language_core::LexicalPart::SeparableParticle && *form == "an"
		})
		.context("missing separable particle")?;
	assert!(particle > stem + 1, "particle should follow an object, got {parts:?}");
	Ok(())
}

#[test]
fn serial_verb_promotes_the_instrument() -> anyhow::Result<()> {
	let (universe, lex) = lex()?;
	let utterance = john_cut_meat_with_knife(&universe, &lex)?;
	let output = render(utterance, &universe);
	let isolating = words(&output, isolating_svo());
	let serial = words(&output, serial_verb());
	assert!(
		isolating.iter().any(|word| word == "with"),
		"isolating grammar should keep an instrumental adposition: {isolating:?}"
	);
	assert!(serial.iter().any(|word| word == "gba"), "serial take: {serial:?}");
	assert!(!serial.iter().any(|word| word == "with"), "serial should drop with: {serial:?}");
	assert_ne!(isolating, serial);
	Ok(())
}

#[test]
fn topic_particle_fronts_the_topic() -> anyhow::Result<()> {
	let (universe, lex) = lex()?;
	let mut utterance = Utterance::john_gave_the_book_to_mary(&lex);
	let john = utterance
		.referents
		.iter()
		.find(|(_, referent)| referent.concept == lex.john)
		.map(|(id, _)| id)
		.context("john")?;
	utterance.information.topic = Some(john);
	utterance.information.focus = Some(FocusTarget::Referent(john));
	let output = render(utterance, &universe);
	let realized = words(&output, particle_heavy_topic_prominent());
	assert!(realized.iter().any(|word| word == "wa"), "topic particle: {realized:?}");
	let john_ipa = output.realization_of(lex.john).context("john term")?.term.ipa.as_str();
	assert_eq!(realized.first().map(String::as_str), Some(john_ipa));
	Ok(())
}

#[test]
fn surface_ir_keeps_semantic_provenance() -> anyhow::Result<()> {
	let (universe, lex) = lex()?;
	let output = render(Utterance::john_gave_the_book_to_mary(&lex), &universe);
	let surface = output.realize_surface(isolating_svo());
	let lexical: Vec<_> = surface
		.clauses
		.iter()
		.flat_map(|clause| clause.constituents.iter())
		.filter_map(|item| match item {
			SurfaceConstituent::Lexical { node, .. } => Some(*node),
			_ => None,
		})
		.collect();
	assert!(lexical.iter().any(|node| matches!(node, SemanticNode::Predicate(_))));
	assert!(lexical.iter().any(|node| matches!(node, SemanticNode::Referent(_))));
	Ok(())
}

#[test]
fn six_word_orders_are_distinct() -> anyhow::Result<()> {
	let (universe, lex) = lex()?;
	let output = render(Utterance::john_gave_the_book_to_mary(&lex), &universe);
	let mut seen = Vec::new();
	for order in [
		WordOrder::Svo,
		WordOrder::Sov,
		WordOrder::Vso,
		WordOrder::Vos,
		WordOrder::Ovs,
		WordOrder::Osv,
	] {
		let mut grammar = isolating_svo();
		grammar.order = order;
		let realized = words(&output, grammar);
		assert!(
			!seen.contains(&realized),
			"order {order:?} collided with an earlier linearization: {realized:?}"
		);
		seen.push(realized);
	}
	Ok(())
}

#[test]
fn basic_surface_grammar_still_matches_the_poc() -> anyhow::Result<()> {
	let (universe, lex) = lex()?;
	let output = render(Utterance::john_gave_the_book_to_mary(&lex), &universe);
	let basic = output.realize(SurfaceGrammar::compositional()).ipa().to_string();
	assert!(basic.starts_with('/'));
	assert!(basic.ends_with('/'));
	assert!(basic.contains(SurfaceGrammar::compositional().particles.recipient));
	Ok(())
}

#[test]
fn alignment_enum_does_not_equate_agent_with_subject() {
	assert_eq!(
		Alignment::NominativeAccusative.relation(SemanticRole::Agent),
		maybraid_language_core::GrammaticalRelation::Subject
	);
	assert_eq!(
		Alignment::ErgativeAbsolutive.relation(SemanticRole::Agent),
		maybraid_language_core::GrammaticalRelation::Oblique
	);
	assert_eq!(
		Alignment::ErgativeAbsolutive.relation(SemanticRole::Theme),
		maybraid_language_core::GrammaticalRelation::Subject
	);
	assert_eq!(
		Alignment::ActiveStative.relation(SemanticRole::Experiencer),
		maybraid_language_core::GrammaticalRelation::Object
	);
	assert_eq!(
		Alignment::Tripartite.relation(SemanticRole::Agent),
		maybraid_language_core::GrammaticalRelation::Oblique
	);
	assert_eq!(
		Alignment::Neutral
			.case_for(maybraid_language_core::GrammaticalRelation::Object, SemanticRole::Theme),
		crate::CaseLabel::Nominative
	);
}

fn mary_is_a_witch(lex: &PocLexicon) -> Utterance {
	let mut utterance = Utterance::new();
	let mary = utterance.add_referent(Referent::new(lex.mary).proper());
	let witch = utterance.add_referent(Referent::new(lex.witch));
	let clause = utterance.add_clause(
		Clause::new(lex.classified_as)
			.with_argument(SemanticRole::Theme, SemanticValue::Referent(mary))
			.with_argument(SemanticRole::Classification, SemanticValue::Referent(witch)),
	);
	utterance.push_root(clause);
	utterance
}

fn john_is_at_the_river(lex: &PocLexicon) -> Utterance {
	let mut utterance = Utterance::new();
	let john = utterance.add_referent(Referent::new(lex.john).proper());
	let river = utterance.add_referent(Referent::new(lex.river).definite());
	let clause = utterance.add_clause(
		Clause::new(lex.classified_as)
			.with_argument(SemanticRole::Theme, SemanticValue::Referent(john))
			.with_argument(SemanticRole::Location, SemanticValue::Referent(river)),
	);
	utterance.push_root(clause);
	utterance
}

fn mary_said_john_gave(
	universe: &impl EnglishSenseLookup,
	lex: &PocLexicon,
) -> anyhow::Result<Utterance> {
	let say = universe.verb_sense("say", 0)?;
	let mut utterance = Utterance::john_gave_the_book_to_mary(lex);
	let give = *utterance.roots.first().context("give")?;
	utterance.roots.clear();
	let mary = utterance
		.referents
		.iter()
		.find(|(_, referent)| referent.concept == lex.mary)
		.map(|(id, _)| id)
		.context("mary")?;
	let said = utterance.add_clause(
		Clause::new(say)
			.with_argument(SemanticRole::Agent, SemanticValue::Referent(mary))
			.with_argument(SemanticRole::Content, SemanticValue::Clause(give)),
	);
	utterance.push_root(said);
	Ok(utterance)
}

#[test]
fn question_particle_marks_interrogative() -> anyhow::Result<()> {
	let (universe, lex) = lex()?;
	let mut utterance = Utterance::john_gave_the_book_to_mary(&lex);
	let root = utterance.roots[0];
	if let Some(clause) = utterance.clauses.get_mut(root) {
		*clause = clause.clone().interrogative();
	}
	let output = render(utterance, &universe);
	let realized = words(&output, isolating_svo());
	assert_eq!(realized.last().map(String::as_str), Some("ma"), "question particle: {realized:?}");
	Ok(())
}

#[test]
fn inversion_fronts_the_predicate() -> anyhow::Result<()> {
	let (universe, lex) = lex()?;
	let mut utterance = Utterance::john_gave_the_book_to_mary(&lex);
	let root = utterance.roots[0];
	if let Some(clause) = utterance.clauses.get_mut(root) {
		*clause = clause.clone().interrogative();
	}
	let output = render(utterance, &universe);
	let svo = words(&output, isolating_svo());
	let inverted = words(&output, fusional_svo());
	assert_eq!(
		inverted.first().map(String::as_str),
		Some("li"),
		"question particle first: {inverted:?}"
	);
	assert_ne!(svo, inverted);
	let give = output
		.ipa_for(SemanticNode::Predicate(output.utterance.roots[0]))
		.context("give ipa")?;
	let verb_at = inverted.iter().position(|word| word == give).context("verb")?;
	let john = output.realization_of(lex.john).context("john")?.term.ipa.as_str();
	let john_at = inverted.iter().position(|word| word == john).context("john")?;
	assert!(verb_at < john_at, "inverted V before S: {inverted:?}");
	Ok(())
}

#[test]
fn zero_copula_drops_the_predicate() -> anyhow::Result<()> {
	let (universe, lex) = lex()?;
	let output = render(mary_is_a_witch(&lex), &universe);
	let zero = words(&output, isolating_svo());
	let mut stative = isolating_svo();
	stative.copula = CopulaStrategy::Stative;
	let kept = words(&output, stative);
	let obligatory = words(&output, fusional_svo());
	assert!(kept.len() > zero.len(), "zero copula should drop the predicate: {zero:?} vs {kept:?}");
	assert!(obligatory.iter().any(|word| word == "est"), "obligatory copula: {obligatory:?}");
	Ok(())
}

#[test]
fn locative_copula_is_not_classification() -> anyhow::Result<()> {
	let (universe, lex) = lex()?;
	let output = render(john_is_at_the_river(&lex), &universe);
	let locative = words(&output, ergative_vso());
	assert!(locative.iter().any(|word| word == "ta"), "locative copula: {locative:?}");
	let classificational = words(&output, fusional_svo());
	assert!(
		classificational.iter().any(|word| word == "est"),
		"classificational uses obligatory form only when Classification is present; locative still verbal or obligatory: {classificational:?}"
	);
	Ok(())
}

#[test]
fn subject_agreement_marks_plural() -> anyhow::Result<()> {
	let (universe, lex) = lex()?;
	let mut utterance = Utterance::john_gave_the_book_to_mary(&lex);
	let john = utterance
		.referents
		.iter()
		.find(|(_, referent)| referent.concept == lex.john)
		.map(|(id, _)| id)
		.context("john")?;
	if let Some(john) = utterance.referents.get_mut(john) {
		john.number = maybraid_language_core::Number::Plural;
		john.person = maybraid_language_core::Person::Third;
	}
	let output = render(utterance, &universe);
	let realized = words(&output, fusional_svo());
	assert!(realized.iter().any(|word| word == "n"), "plural agreement: {realized:?}");
	assert!(realized.iter().any(|word| word == "t"), "person agreement: {realized:?}");
	Ok(())
}

#[test]
fn complementizer_wraps_embedded_clause() -> anyhow::Result<()> {
	let (universe, lex) = lex()?;
	let utterance = mary_said_john_gave(&universe, &lex)?;
	let output = render(utterance, &universe);
	let isolating = words(&output, isolating_svo());
	let converb = words(&output, agglutinative_sov());
	let subordinate = words(&output, particle_heavy_topic_prominent());
	assert!(isolating.iter().any(|word| word == "shuo"), "complementizer: {isolating:?}");
	assert!(converb.iter().any(|word| word == "te"), "converb: {converb:?}");
	assert!(subordinate.iter().any(|word| word == "to"), "subordinator: {subordinate:?}");
	Ok(())
}

#[test]
fn coordinator_joins_root_clauses() -> anyhow::Result<()> {
	let (universe, lex) = lex()?;
	let mut utterance = Utterance::john_gave_the_book_to_mary(&lex);
	let mary = utterance
		.referents
		.iter()
		.find(|(_, referent)| referent.concept == lex.mary)
		.map(|(id, _)| id)
		.context("mary")?;
	let witch = utterance.add_referent(Referent::new(lex.witch));
	let classified = utterance.add_clause(
		Clause::new(lex.classified_as)
			.with_argument(SemanticRole::Theme, SemanticValue::Referent(mary))
			.with_argument(SemanticRole::Classification, SemanticValue::Referent(witch)),
	);
	utterance.push_root(classified);
	let output = render(utterance, &universe);
	let realized = words(&output, fusional_svo());
	assert!(realized.iter().any(|word| word == "et"), "coordinator: {realized:?}");
	Ok(())
}
