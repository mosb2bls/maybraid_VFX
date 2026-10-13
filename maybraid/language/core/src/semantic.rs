//! English UD → [`Utterance`] marshalling.

use std::collections::HashMap;

use crate::concept::{ConceptId, ConceptUniverse, Pos};
use crate::error::LanguageError;
use crate::parse::{
	DependencyDocument, DependencySentence, DependencyToken, TokenId, UniversalPos,
};
use crate::utterance::{
	Aspect, Clause, Definiteness, Modifier, Mood, Number, Person, Polarity, Referent, ReferentId,
	SemanticRole, SemanticValue, Tense, Utterance,
};
use crate::wordnet::normalize_lemma;

/// Syntax → semantic graph.
pub trait SemanticMarshaller {
	fn marshal(
		&self,
		document: &DependencyDocument,
		universe: &impl ConceptUniverse,
	) -> Result<Utterance, LanguageError>;
}

/// Curated predicate-frame lookup for the POC dialogue set.
#[derive(Clone, Debug, Default)]
pub struct PredicateFrameLexicon {
	frames: Vec<PredicateFrame>,
}

#[derive(Clone, Debug)]
pub struct PredicateFrame {
	pub lemma: &'static str,
	pub mappings: Vec<RoleMapping>,
}

#[derive(Clone, Copy, Debug)]
pub struct RoleMapping {
	pub relation: &'static str,
	pub case: Option<&'static str>,
	pub role: SemanticRole,
}

impl PredicateFrameLexicon {
	pub fn poc() -> Self {
		Self {
			frames: vec![
				frame(
					"give",
					&[
						map("nsubj", None, SemanticRole::Agent),
						map("obj", None, SemanticRole::Theme),
						map("iobj", None, SemanticRole::Recipient),
						map("obl", Some("to"), SemanticRole::Recipient),
					],
				),
				frame(
					"see",
					&[
						map("nsubj", None, SemanticRole::Experiencer),
						map("obj", None, SemanticRole::Theme),
					],
				),
				frame("fall", &[map("nsubj", None, SemanticRole::Theme)]),
				frame(
					"go",
					&[
						map("nsubj", None, SemanticRole::Agent),
						map("obl", Some("to"), SemanticRole::Goal),
						map("obl", Some("for"), SemanticRole::Goal),
						map("obl", Some("from"), SemanticRole::Source),
					],
				),
				frame(
					"walk",
					&[
						map("nsubj", None, SemanticRole::Agent),
						map("obl", Some("to"), SemanticRole::Goal),
						map("obl", Some("from"), SemanticRole::Source),
						map("advmod", None, SemanticRole::Manner),
					],
				),
				frame(
					"tell",
					&[
						map("nsubj", None, SemanticRole::Agent),
						map("obj", None, SemanticRole::Recipient),
						map("iobj", None, SemanticRole::Recipient),
						map("obl", Some("to"), SemanticRole::Recipient),
						map("ccomp", None, SemanticRole::Content),
					],
				),
				frame(
					"steal",
					&[
						map("nsubj", None, SemanticRole::Agent),
						map("obj", None, SemanticRole::Theme),
					],
				),
				frame("leave", &[map("nsubj", None, SemanticRole::Agent)]),
				frame(
					"be",
					&[
						map("nsubj", None, SemanticRole::Theme),
						map("xcomp", None, SemanticRole::Classification),
						map("attr", None, SemanticRole::Classification),
					],
				),
			],
		}
	}

	pub fn mappings_for(&self, lemma: &str) -> &[RoleMapping] {
		self.frames
			.iter()
			.find(|frame| frame.lemma == lemma)
			.map(|frame| frame.mappings.as_slice())
			.unwrap_or(&[])
	}
}

fn frame(lemma: &'static str, mappings: &[RoleMapping]) -> PredicateFrame {
	PredicateFrame { lemma, mappings: mappings.to_vec() }
}

const fn map(
	relation: &'static str,
	case: Option<&'static str>,
	role: SemanticRole,
) -> RoleMapping {
	RoleMapping { relation, case, role }
}

/// Converts a [`DependencyDocument`] into an [`Utterance`].
#[derive(Clone, Debug)]
pub struct EnglishSemanticMarshaller {
	pub predicate_frames: PredicateFrameLexicon,
}

impl Default for EnglishSemanticMarshaller {
	fn default() -> Self {
		Self { predicate_frames: PredicateFrameLexicon::poc() }
	}
}

impl SemanticMarshaller for EnglishSemanticMarshaller {
	fn marshal(
		&self,
		document: &DependencyDocument,
		universe: &impl ConceptUniverse,
	) -> Result<Utterance, LanguageError> {
		let mut utterance = Utterance::new();
		for sentence in &document.sentences {
			self.marshal_sentence(sentence, universe, &mut utterance)?;
		}
		if utterance.roots.is_empty() && !utterance.clauses.is_empty() {
			if let Some((id, _)) = utterance.clauses.iter().next() {
				utterance.push_root(id);
			}
		}
		Ok(utterance)
	}
}

impl EnglishSemanticMarshaller {
	fn marshal_sentence(
		&self,
		sentence: &DependencySentence,
		universe: &impl ConceptUniverse,
		utterance: &mut Utterance,
	) -> Result<(), LanguageError> {
		let predicates = predicate_tokens(sentence);
		let mut clauses = HashMap::<TokenId, crate::utterance::ClauseId>::new();
		for predicate in &predicates {
			let lemma = normalize_lemma(&predicate.lemma);
			let concept = if is_copular(predicate, sentence) {
				resolve_concept(universe, "classified_as", Some(Pos::Verb))?
			} else {
				resolve_predicate(universe, &lemma)?
			};
			let mut clause = Clause::new(concept);
			apply_clause_morph(&mut clause, predicate, sentence);
			clauses.insert(predicate.id, utterance.add_clause(clause));
		}

		let mut referents = HashMap::<TokenId, ReferentId>::new();
		for predicate in &predicates {
			let Some(&clause_id) = clauses.get(&predicate.id) else {
				continue;
			};
			let lemma = normalize_lemma(&predicate.lemma);
			let frame_lemma = if is_copular(predicate, sentence) { "be" } else { lemma.as_str() };
			if is_copular(predicate, sentence) {
				let classification =
					self.ensure_referent(predicate, sentence, universe, utterance, &mut referents)?;
				if let Some(clause) = utterance.clauses.get_mut(clause_id) {
					if !clause
						.arguments
						.iter()
						.any(|argument| argument.role == SemanticRole::Classification)
					{
						clause.arguments.push(crate::utterance::Argument {
							role: SemanticRole::Classification,
							value: SemanticValue::Referent(classification),
						});
					}
				}
			}
			for dependent in sentence.children(predicate.id) {
				if is_universal_quantifier(dependent) && !is_core_argument(dependent) {
					if let Some(subject) =
						sentence.children(predicate.id).find(|child| child.relation.is("nsubj"))
					{
						if let Ok(referent) = self.try_ensure_referent(
							subject,
							sentence,
							universe,
							utterance,
							&mut referents,
						) {
							if let Some(stored) = utterance.referents.get_mut(referent) {
								stored.number = Number::Many;
							}
						}
					}
					continue;
				}
				if should_skip_dependent(dependent) {
					continue;
				}
				if let Some(nested) = clauses.get(&dependent.id).copied() {
					if let Some(role) = self.role_for(frame_lemma, predicate, dependent, sentence) {
						if let Some(clause) = utterance.clauses.get_mut(clause_id) {
							clause.arguments.push(crate::utterance::Argument {
								role,
								value: SemanticValue::Clause(nested),
							});
						}
					}
					continue;
				}
				let Some(nominal) = argument_head(dependent, sentence) else {
					continue;
				};
				let core = is_core_argument(dependent);
				let referent = match self.try_ensure_referent(
					nominal,
					sentence,
					universe,
					utterance,
					&mut referents,
				) {
					Ok(referent) => referent,
					Err(error) if core => return Err(error),
					Err(_) => continue,
				};
				apply_quantifiers(nominal, sentence, utterance, referent);
				let Some(role) = self.role_for(frame_lemma, predicate, dependent, sentence) else {
					continue;
				};
				if let Some(clause) = utterance.clauses.get_mut(clause_id) {
					if !clause.arguments.iter().any(|argument| argument.role == role) {
						clause.arguments.push(crate::utterance::Argument {
							role,
							value: SemanticValue::Referent(referent),
						});
					}
				}
			}
		}

		for token in &sentence.tokens {
			if token.relation.base() != "acl" {
				continue;
			}
			let Some(&clause_id) = clauses.get(&token.id) else {
				continue;
			};
			let Some(head) = token.head.and_then(|id| sentence.token(id)) else {
				continue;
			};
			let referent =
				self.ensure_referent(head, sentence, universe, utterance, &mut referents)?;
			if let Some(stored) = utterance.referents.get_mut(referent) {
				if !stored.relative_clauses.contains(&clause_id) {
					stored.relative_clauses.push(clause_id);
				}
			}
		}

		for token in &sentence.tokens {
			if token.relation.is("root") {
				if let Some(&clause_id) = clauses.get(&token.id) {
					utterance.push_root(clause_id);
				} else if let Some(verb) = sentence
					.children(token.id)
					.find(|child| matches!(child.pos, UniversalPos::Verb))
				{
					if let Some(&clause_id) = clauses.get(&verb.id) {
						utterance.push_root(clause_id);
					}
				}
			}
		}
		Ok(())
	}

	fn ensure_referent(
		&self,
		token: &DependencyToken,
		sentence: &DependencySentence,
		universe: &impl ConceptUniverse,
		utterance: &mut Utterance,
		referents: &mut HashMap<TokenId, ReferentId>,
	) -> Result<ReferentId, LanguageError> {
		if let Some(id) = referents.get(&token.id) {
			return Ok(*id);
		}
		if is_relative_pronoun(token) {
			if let Some(antecedent) = relative_antecedent(token, sentence) {
				let id =
					self.ensure_referent(antecedent, sentence, universe, utterance, referents)?;
				referents.insert(token.id, id);
				return Ok(id);
			}
		}
		let lemma = referent_lemma(token);
		let prefer = match token.pos {
			UniversalPos::Adjective => Some(Pos::Adjective),
			UniversalPos::Adverb => Some(Pos::Adverb),
			UniversalPos::Verb => Some(Pos::Verb),
			UniversalPos::Numeral => Some(Pos::Noun),
			_ => Some(Pos::Noun),
		};
		let concept = resolve_concept(universe, &lemma, prefer)?;
		let mut referent = Referent::new(concept);
		referent.definiteness = definiteness_of(token, sentence);
		referent.number = number_of(token, sentence);
		referent.person = person_of(token);
		for child in sentence.children(token.id) {
			if child.relation.is("amod") {
				if let Ok(modifier) = resolve_concept(universe, &child.lemma, Some(Pos::Adjective))
				{
					referent.modifiers.push(Modifier { concept: modifier });
				}
			}
		}
		if is_universal_quantifier(token) || is_universal_quantified(token, sentence) {
			referent.number = Number::Many;
		}
		let id = utterance.add_referent(referent);
		referents.insert(token.id, id);
		Ok(id)
	}

	fn try_ensure_referent(
		&self,
		token: &DependencyToken,
		sentence: &DependencySentence,
		universe: &impl ConceptUniverse,
		utterance: &mut Utterance,
		referents: &mut HashMap<TokenId, ReferentId>,
	) -> Result<ReferentId, LanguageError> {
		self.ensure_referent(token, sentence, universe, utterance, referents)
	}

	fn role_for(
		&self,
		lemma: &str,
		predicate: &DependencyToken,
		dependent: &DependencyToken,
		sentence: &DependencySentence,
	) -> Option<SemanticRole> {
		if is_passive(predicate, sentence) {
			if dependent.relation.is("nsubj:pass") || dependent.relation.is("nsubj") {
				return Some(SemanticRole::Theme);
			}
			if dependent.relation.is("obl") && case_of(dependent, sentence) == Some("by") {
				return Some(SemanticRole::Agent);
			}
		}
		let case = case_of(dependent, sentence);
		let rel = dependent.relation.as_str();
		let base = dependent.relation.base();
		let mappings = self.predicate_frames.mappings_for(lemma);
		if let Some(mapping) = mappings
			.iter()
			.find(|mapping| relation_matches(mapping.relation, rel, base) && mapping.case == case)
		{
			return Some(mapping.role);
		}
		if let Some(mapping) = mappings
			.iter()
			.find(|mapping| relation_matches(mapping.relation, rel, base) && mapping.case.is_none())
		{
			return Some(mapping.role);
		}
		oblique_role(dependent, case, sentence).or_else(|| default_role(dependent, sentence))
	}
}

fn predicate_tokens(sentence: &DependencySentence) -> Vec<&DependencyToken> {
	sentence.tokens.iter().filter(|token| is_predicate(token, sentence)).collect()
}

fn is_predicate(token: &DependencyToken, sentence: &DependencySentence) -> bool {
	if matches!(token.pos, UniversalPos::Verb) {
		return true;
	}
	if token.relation.is("root") {
		if matches!(token.pos, UniversalPos::Auxiliary) {
			return !sentence
				.children(token.id)
				.any(|child| matches!(child.pos, UniversalPos::Verb));
		}
		// UDPipe often tags eventive "walk" as a noun root, with PPs as nmod.
		return !matches!(
			token.pos,
			UniversalPos::Punctuation
				| UniversalPos::Adposition
				| UniversalPos::Determiner
				| UniversalPos::Particle
		);
	}
	false
}

fn relation_matches(mapping: &str, rel: &str, base: &str) -> bool {
	mapping == rel
		|| mapping == base
		|| (mapping == "obl" && (base == "nmod" || rel == "nmod"))
		|| (mapping == "nmod" && (base == "obl" || rel == "obl"))
}

fn should_skip_dependent(token: &DependencyToken) -> bool {
	if token.relation.is("neg") {
		return false;
	}
	if token.relation.is("advmod") {
		return is_negation(token);
	}
	matches!(
		token.relation.base(),
		"aux" | "cop" | "mark" | "punct" | "case" | "det" | "cc" | "expl"
	)
}

fn is_core_argument(token: &DependencyToken) -> bool {
	matches!(token.relation.base(), "nsubj" | "obj" | "iobj")
}

fn argument_head<'a>(
	dependent: &'a DependencyToken,
	sentence: &'a DependencySentence,
) -> Option<&'a DependencyToken> {
	if is_nominal(dependent)
		|| is_universal_quantifier(dependent)
		|| matches!(dependent.pos, UniversalPos::Adverb | UniversalPos::Adjective)
	{
		return Some(dependent);
	}
	sentence
		.children(dependent.id)
		.find(|child| is_nominal(child) || is_universal_quantifier(child))
}

fn is_universal_quantifier(token: &DependencyToken) -> bool {
	matches!(normalize_lemma(&token.lemma).as_str(), "all" | "both" | "every")
}

fn is_universal_quantified(token: &DependencyToken, sentence: &DependencySentence) -> bool {
	sentence.children(token.id).any(is_universal_quantifier)
}

fn apply_quantifiers(
	token: &DependencyToken,
	sentence: &DependencySentence,
	utterance: &mut Utterance,
	referent: ReferentId,
) {
	if is_universal_quantified(token, sentence) {
		if let Some(stored) = utterance.referents.get_mut(referent) {
			stored.number = Number::Many;
		}
	}
}

fn fold_apostrophes(text: &str) -> String {
	text.replace(['\u{2019}', '\u{2018}', '\u{02BC}', '`'], "'")
}

fn is_negation(token: &DependencyToken) -> bool {
	if token.relation.is("neg") || token.features.has("Polarity", "Neg") {
		return true;
	}
	let lemma = fold_apostrophes(&normalize_lemma(&token.lemma));
	let text = fold_apostrophes(&normalize_lemma(&token.text));
	lemma == "not"
		|| text == "not"
		|| text == "n't"
		|| text.ends_with("n't")
		|| matches!(text.as_str(), "don't" | "doesn't" | "didn't" | "won't" | "can't" | "cannot")
}

fn clause_has_negation(predicate: &DependencyToken, sentence: &DependencySentence) -> bool {
	if is_negation(predicate) {
		return true;
	}
	sentence.tokens.iter().any(|token| {
		if !is_negation(token) {
			return false;
		}
		let Some(head) = token.head else {
			return false;
		};
		head == predicate.id
			|| sentence.token(head).is_some_and(|parent| {
				parent.head == Some(predicate.id) && (is_aux(parent) || is_cop(parent))
			})
	})
}

fn is_nominal(token: &DependencyToken) -> bool {
	matches!(
		token.pos,
		UniversalPos::Noun
			| UniversalPos::ProperNoun
			| UniversalPos::Pronoun
			| UniversalPos::Numeral
	)
}

fn is_relative_pronoun(token: &DependencyToken) -> bool {
	if token.relation.is("mark") || token.relation.is("det") {
		return false;
	}
	matches!(normalize_lemma(&token.lemma).as_str(), "who" | "that" | "which" | "whom")
}

fn relative_antecedent<'a>(
	token: &DependencyToken,
	sentence: &'a DependencySentence,
) -> Option<&'a DependencyToken> {
	let head = token.head.and_then(|id| sentence.token(id))?;
	if head.relation.base() == "acl" {
		if let Some(noun) = head.head.and_then(|id| sentence.token(id)) {
			if noun.id != token.id {
				return Some(noun);
			}
		}
	}
	sentence.children(head.id).find(|sibling| {
		sibling.id != token.id
			&& is_core_argument(sibling)
			&& !is_relative_pronoun(sibling)
			&& (is_nominal(sibling) || is_universal_quantifier(sibling))
	})
}

fn is_aux(token: &DependencyToken) -> bool {
	token.relation.base() == "aux"
}

fn is_cop(token: &DependencyToken) -> bool {
	token.relation.base() == "cop"
}

fn is_copular(predicate: &DependencyToken, sentence: &DependencySentence) -> bool {
	sentence.children(predicate.id).any(is_cop)
}

fn cardinal_word(lemma: &str) -> Option<&'static str> {
	const ONES: [&str; 20] = [
		"zero",
		"one",
		"two",
		"three",
		"four",
		"five",
		"six",
		"seven",
		"eight",
		"nine",
		"ten",
		"eleven",
		"twelve",
		"thirteen",
		"fourteen",
		"fifteen",
		"sixteen",
		"seventeen",
		"eighteen",
		"nineteen",
	];
	const TENS: [&str; 10] =
		["", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety"];
	let n: u16 = lemma.parse().ok()?;
	match n {
		0..=19 => Some(ONES[usize::from(n)]),
		20 | 30 | 40 | 50 | 60 | 70 | 80 | 90 => Some(TENS[usize::from(n / 10)]),
		_ => None,
	}
}

fn form_is(token: &DependencyToken, form: &str) -> bool {
	normalize_lemma(&token.lemma) == form || normalize_lemma(&token.text) == form
}

fn form_in(token: &DependencyToken, forms: &[&str]) -> bool {
	forms.iter().any(|form| form_is(token, form))
}

fn is_past_aux(token: &DependencyToken) -> bool {
	token.features.has("Tense", "Past") || form_in(token, &["did", "was", "were", "had"])
}

fn is_present_aux(token: &DependencyToken) -> bool {
	token.features.has("Tense", "Pres") || form_in(token, &["am", "is", "are", "do", "does"])
}

fn is_passive(predicate: &DependencyToken, sentence: &DependencySentence) -> bool {
	predicate.features.has("Voice", "Pass")
		|| sentence.children(predicate.id).any(|child| child.relation.is("aux:pass"))
}

fn case_of<'a>(token: &'a DependencyToken, sentence: &'a DependencySentence) -> Option<&'a str> {
	sentence
		.children(token.id)
		.find(|child| child.relation.is("case"))
		.map(|child| child.lemma.as_str())
}

fn oblique_role(
	dependent: &DependencyToken,
	case: Option<&str>,
	_sentence: &DependencySentence,
) -> Option<SemanticRole> {
	if !matches!(dependent.relation.base(), "obl" | "nmod") {
		return None;
	}
	match case {
		Some("to") | Some("into") | Some("toward") | Some("towards") => Some(SemanticRole::Goal),
		Some("for") => Some(SemanticRole::Goal),
		Some("from") => Some(SemanticRole::Source),
		Some("with") => Some(SemanticRole::Instrument),
		Some("at") => Some(at_role(dependent)),
		Some("in") | Some("on") if is_time_nominal(dependent) => Some(SemanticRole::Time),
		Some("in") | Some("on") => Some(SemanticRole::Location),
		Some("by") => None,
		Some(_) => None,
		None => None,
	}
}

fn at_role(dependent: &DependencyToken) -> SemanticRole {
	if is_rate_nominal(dependent) {
		SemanticRole::Rate
	} else if is_time_nominal(dependent) {
		SemanticRole::Time
	} else {
		SemanticRole::Location
	}
}

fn is_rate_nominal(token: &DependencyToken) -> bool {
	matches!(
		normalize_lemma(&token.lemma).as_str(),
		"speed" | "rate" | "pace" | "velocity" | "tempo"
	)
}

fn is_time_nominal(token: &DependencyToken) -> bool {
	matches!(
		normalize_lemma(&token.lemma).as_str(),
		"noon"
			| "midnight"
			| "dawn" | "dusk"
			| "morning"
			| "evening"
			| "night" | "today"
			| "tomorrow"
			| "yesterday"
			| "time"
	)
}

fn default_role(
	dependent: &DependencyToken,
	sentence: &DependencySentence,
) -> Option<SemanticRole> {
	if dependent.relation.is("advcl") {
		return adverbial_clause_role(dependent, sentence);
	}
	match dependent.relation.as_str() {
		"nsubj:pass" => Some(SemanticRole::Theme),
		"nsubj" => Some(SemanticRole::Agent),
		"obj" => Some(SemanticRole::Theme),
		"iobj" => Some(SemanticRole::Recipient),
		"ccomp" | "xcomp" => Some(SemanticRole::Content),
		"advmod" => Some(SemanticRole::Manner),
		_ => None,
	}
}

fn adverbial_clause_role(
	dependent: &DependencyToken,
	sentence: &DependencySentence,
) -> Option<SemanticRole> {
	let mark = sentence
		.children(dependent.id)
		.find(|child| child.relation.is("mark"))
		.map(|child| normalize_lemma(&child.lemma));
	Some(match mark.as_deref() {
		Some("when") | Some("while") | Some("before") | Some("after") | Some("as")
		| Some("until") => SemanticRole::Time,
		_ => SemanticRole::Cause,
	})
}

fn apply_clause_morph(
	clause: &mut Clause,
	predicate: &DependencyToken,
	sentence: &DependencySentence,
) {
	if predicate.features.has("Tense", "Past")
		|| sentence.children(predicate.id).any(|child| is_aux(child) && is_past_aux(child))
	{
		clause.tense = Tense::Past;
	} else if sentence
		.children(predicate.id)
		.any(|child| is_aux(child) && form_is(child, "will"))
	{
		clause.tense = Tense::Future;
	} else if predicate.features.has("Tense", "Pres")
		|| sentence
			.children(predicate.id)
			.any(|child| is_aux(child) && is_present_aux(child))
	{
		clause.tense = Tense::Present;
	}

	if !is_passive(predicate, sentence)
		&& (predicate.features.has("VerbForm", "Ger")
			|| predicate.features.has("VerbForm", "Part")
			|| sentence.children(predicate.id).any(|child| {
				is_aux(child) && form_in(child, &["am", "is", "are", "was", "were", "be"])
			})) {
		clause.aspect = Aspect::Progressive;
	}
	if sentence
		.children(predicate.id)
		.any(|child| form_in(child, &["have", "has", "had"]))
	{
		clause.aspect = Aspect::Perfect;
	}

	if clause_has_negation(predicate, sentence) {
		clause.polarity = Polarity::Negative;
	}
	if predicate.features.has("Mood", "Imp") {
		clause.mood = Mood::Imperative;
	}
}

fn referent_lemma(token: &DependencyToken) -> String {
	match normalize_lemma(&token.lemma).as_str() {
		"i" | "me" | "we" | "us" | "myself" => "speaker".to_owned(),
		"you" => "listener".to_owned(),
		"he" | "him" | "his" | "she" | "her" | "they" | "them" | "their" | "himself"
		| "herself" | "themselves" | "who" | "whom" => "person".to_owned(),
		"it" | "its" | "itself" | "that" | "which" | "all" | "both" | "every" => "thing".to_owned(),
		other => other.to_owned(),
	}
}

fn definiteness_of(token: &DependencyToken, sentence: &DependencySentence) -> Definiteness {
	if matches!(token.pos, UniversalPos::ProperNoun) {
		return Definiteness::Proper;
	}
	for child in sentence.children(token.id) {
		if child.relation.is("det") {
			return match normalize_lemma(&child.lemma).as_str() {
				"the" | "this" | "that" | "these" | "those" => Definiteness::Definite,
				_ => Definiteness::Indefinite,
			};
		}
	}
	if matches!(
		normalize_lemma(&token.lemma).as_str(),
		"speaker" | "listener" | "person" | "thing" | "i" | "you" | "he" | "she" | "they" | "it"
	) {
		Definiteness::Definite
	} else {
		Definiteness::Indefinite
	}
}

fn person_of(token: &DependencyToken) -> Person {
	match token.features.get("Person") {
		Some("1") => Person::First,
		Some("2") => Person::Second,
		Some("3") => Person::Third,
		_ => match token.lemma.to_ascii_lowercase().as_str() {
			"i" | "me" | "we" | "us" | "myself" => Person::First,
			"you" => Person::Second,
			"he" | "him" | "she" | "her" | "they" | "them" => Person::Third,
			_ => Person::Unspecified,
		},
	}
}

fn number_of(token: &DependencyToken, sentence: &DependencySentence) -> Number {
	if token.features.has("Number", "Plur") {
		return Number::Plural;
	}
	if sentence.children(token.id).any(|child| child.relation.is("nummod")) {
		return Number::Many;
	}
	Number::Singular
}

fn resolve_predicate(
	universe: &impl ConceptUniverse,
	lemma: &str,
) -> Result<ConceptId, LanguageError> {
	if matches!(lemma, "be" | "am" | "is" | "are" | "was" | "were") {
		return resolve_concept(universe, "be", Some(Pos::Verb));
	}
	resolve_concept(universe, lemma, Some(Pos::Verb))
}

fn resolve_concept(
	universe: &impl ConceptUniverse,
	name: &str,
	prefer: Option<Pos>,
) -> Result<ConceptId, LanguageError> {
	let lemma = normalize_lemma(name);
	let aliases = match lemma.as_str() {
		"i" | "me" | "we" | "us" | "myself" | "speaker" | "self" => {
			vec!["speaker".to_owned(), "person".to_owned()]
		}
		"you" | "listener" => vec!["listener".to_owned(), "person".to_owned()],
		"he" | "him" | "his" | "she" | "her" | "they" | "them" | "their" | "himself"
		| "herself" | "themselves" | "who" | "whom" | "person" => vec!["person".to_owned()],
		"it" | "its" | "itself" | "that" | "which" | "all" | "both" | "every" | "thing" => {
			vec!["thing".to_owned()]
		}
		other => {
			let mut aliases = vec![other.to_owned()];
			if let Some(word) = cardinal_word(other) {
				aliases.push(word.to_owned());
			}
			aliases
		}
	};
	for alias in aliases {
		let ids = universe.resolve_english(&alias);
		if let Some(id) = pick_concept(&ids, prefer) {
			return Ok(id);
		}
	}
	Err(LanguageError::MissingConcept(name.to_owned()))
}

fn pick_concept(ids: &[ConceptId], prefer: Option<Pos>) -> Option<ConceptId> {
	if let Some(pos) = prefer {
		if let Some(id) = ids.iter().copied().find(|id| id.pos() == Some(pos)) {
			return Some(id);
		}
	}
	ids.first().copied()
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::parse::{DependencyRelation, MorphFeatures, TokenId};
	use crate::poc::poc_universe;

	fn tok(
		id: u32,
		text: &str,
		lemma: &str,
		pos: UniversalPos,
		head: u32,
		rel: &str,
		feats: &str,
	) -> DependencyToken {
		DependencyToken {
			id: TokenId(id),
			text: text.to_owned(),
			lemma: lemma.to_owned(),
			pos,
			features: MorphFeatures::parse(feats),
			head: if head == 0 { None } else { Some(TokenId(head)) },
			relation: DependencyRelation::parse(rel),
		}
	}

	fn sentence(tokens: Vec<DependencyToken>) -> DependencyDocument {
		DependencyDocument { sentences: vec![DependencySentence { tokens }] }
	}

	fn universe() -> Result<crate::wordnet::WordNetConceptUniverse, LanguageError> {
		Ok(poc_universe()?.with_proper_names(["John", "Mary", "Alice", "speaker", "listener"]))
	}

	fn universe_for(
		doc: &DependencyDocument,
	) -> Result<crate::wordnet::WordNetConceptUniverse, LanguageError> {
		Ok(universe()?.with_document_overlays(doc))
	}

	fn first_clause(utterance: &Utterance) -> Result<&Clause, LanguageError> {
		utterance
			.clauses
			.values()
			.next()
			.ok_or_else(|| LanguageError::SemanticMarshal("missing clause".to_owned()))
	}

	fn roles(utterance: &Utterance) -> Vec<SemanticRole> {
		utterance
			.clauses
			.values()
			.next()
			.map(|clause| clause.arguments.iter().map(|argument| argument.role).collect())
			.unwrap_or_default()
	}

	fn has_role(utterance: &Utterance, role: SemanticRole) -> bool {
		utterance
			.clauses
			.values()
			.any(|clause| clause.arguments.iter().any(|argument| argument.role == role))
	}

	#[test]
	fn john_gave_mary_the_book() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "John", "John", UniversalPos::ProperNoun, 2, "nsubj", ""),
			tok(2, "gave", "give", UniversalPos::Verb, 0, "root", "Tense=Past"),
			tok(3, "Mary", "Mary", UniversalPos::ProperNoun, 2, "iobj", ""),
			tok(4, "the", "the", UniversalPos::Determiner, 5, "det", ""),
			tok(5, "book", "book", UniversalPos::Noun, 2, "obj", ""),
			tok(6, ".", ".", UniversalPos::Punctuation, 2, "punct", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe()?)?;
		assert!(has_role(&utterance, SemanticRole::Agent));
		assert!(has_role(&utterance, SemanticRole::Theme));
		assert!(has_role(&utterance, SemanticRole::Recipient));
		assert_eq!(utterance.roots.len(), 1);
		Ok(())
	}

	#[test]
	fn john_gave_the_book_to_mary() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "John", "John", UniversalPos::ProperNoun, 2, "nsubj", ""),
			tok(2, "gave", "give", UniversalPos::Verb, 0, "root", "Tense=Past"),
			tok(3, "the", "the", UniversalPos::Determiner, 4, "det", ""),
			tok(4, "book", "book", UniversalPos::Noun, 2, "obj", ""),
			tok(5, "to", "to", UniversalPos::Adposition, 6, "case", ""),
			tok(6, "Mary", "Mary", UniversalPos::ProperNoun, 2, "obl", ""),
			tok(7, ".", ".", UniversalPos::Punctuation, 2, "punct", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe()?)?;
		assert!(has_role(&utterance, SemanticRole::Recipient));
		Ok(())
	}

	#[test]
	fn mary_saw_john() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "Mary", "Mary", UniversalPos::ProperNoun, 2, "nsubj", ""),
			tok(2, "saw", "see", UniversalPos::Verb, 0, "root", "Tense=Past"),
			tok(3, "John", "John", UniversalPos::ProperNoun, 2, "obj", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe()?)?;
		assert!(roles(&utterance).contains(&SemanticRole::Experiencer));
		assert!(roles(&utterance).contains(&SemanticRole::Theme));
		Ok(())
	}

	#[test]
	fn book_was_given_to_mary_by_john() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "The", "the", UniversalPos::Determiner, 2, "det", ""),
			tok(2, "book", "book", UniversalPos::Noun, 4, "nsubj:pass", ""),
			tok(3, "was", "be", UniversalPos::Auxiliary, 4, "aux:pass", "Tense=Past"),
			tok(4, "given", "give", UniversalPos::Verb, 0, "root", "Voice=Pass"),
			tok(5, "to", "to", UniversalPos::Adposition, 6, "case", ""),
			tok(6, "Mary", "Mary", UniversalPos::ProperNoun, 4, "obl", ""),
			tok(7, "by", "by", UniversalPos::Adposition, 8, "case", ""),
			tok(8, "John", "John", UniversalPos::ProperNoun, 4, "obl", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe()?)?;
		assert!(has_role(&utterance, SemanticRole::Theme));
		assert!(has_role(&utterance, SemanticRole::Agent));
		assert!(has_role(&utterance, SemanticRole::Recipient));
		assert_ne!(first_clause(&utterance)?.aspect, Aspect::Progressive);
		Ok(())
	}

	#[test]
	fn mary_told_john_that_alice_left() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "Mary", "Mary", UniversalPos::ProperNoun, 2, "nsubj", ""),
			tok(2, "told", "tell", UniversalPos::Verb, 0, "root", "Tense=Past"),
			tok(3, "John", "John", UniversalPos::ProperNoun, 2, "obj", ""),
			tok(4, "that", "that", UniversalPos::SubordinatingConjunction, 6, "mark", ""),
			tok(5, "Alice", "Alice", UniversalPos::ProperNoun, 6, "nsubj", ""),
			tok(6, "left", "leave", UniversalPos::Verb, 2, "ccomp", "Tense=Past"),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe()?)?;
		assert_eq!(utterance.clauses.len(), 2);
		assert!(has_role(&utterance, SemanticRole::Content));
		Ok(())
	}

	#[test]
	fn mary_saw_the_man_who_stole_the_book() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "Mary", "Mary", UniversalPos::ProperNoun, 2, "nsubj", ""),
			tok(2, "saw", "see", UniversalPos::Verb, 0, "root", "Tense=Past"),
			tok(3, "the", "the", UniversalPos::Determiner, 4, "det", ""),
			tok(4, "man", "man", UniversalPos::Noun, 2, "obj", ""),
			tok(5, "who", "who", UniversalPos::Pronoun, 6, "nsubj", ""),
			tok(6, "stole", "steal", UniversalPos::Verb, 4, "acl:relcl", "Tense=Past"),
			tok(7, "the", "the", UniversalPos::Determiner, 8, "det", ""),
			tok(8, "book", "book", UniversalPos::Noun, 6, "obj", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe()?)?;
		assert_eq!(utterance.clauses.len(), 2);
		assert!(utterance
			.referents
			.values()
			.any(|referent| !referent.relative_clauses.is_empty()));
		Ok(())
	}

	#[test]
	fn john_is_walking_to_the_river() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "John", "John", UniversalPos::ProperNoun, 3, "nsubj", ""),
			tok(2, "is", "be", UniversalPos::Auxiliary, 3, "aux", "Tense=Pres"),
			tok(3, "walking", "walk", UniversalPos::Verb, 0, "root", "VerbForm=Ger"),
			tok(4, "to", "to", UniversalPos::Adposition, 6, "case", ""),
			tok(5, "the", "the", UniversalPos::Determiner, 6, "det", ""),
			tok(6, "river", "river", UniversalPos::Noun, 3, "obl", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe()?)?;
		let clause = first_clause(&utterance)?;
		assert_eq!(clause.aspect, Aspect::Progressive);
		assert_eq!(clause.tense, Tense::Present);
		assert!(has_role(&utterance, SemanticRole::Goal));
		Ok(())
	}

	#[test]
	fn john_did_not_see_mary() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "John", "John", UniversalPos::ProperNoun, 4, "nsubj", ""),
			tok(2, "did", "do", UniversalPos::Auxiliary, 4, "aux", "Tense=Past"),
			tok(3, "not", "not", UniversalPos::Particle, 4, "advmod", "Polarity=Neg"),
			tok(4, "see", "see", UniversalPos::Verb, 0, "root", ""),
			tok(5, "Mary", "Mary", UniversalPos::ProperNoun, 4, "obj", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe()?)?;
		let clause = first_clause(&utterance)?;
		assert_eq!(clause.polarity, Polarity::Negative);
		assert_eq!(clause.tense, Tense::Past);
		Ok(())
	}

	#[test]
	fn i_dont_know_reads_negation_on_the_dummy_aux() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "I", "I", UniversalPos::Pronoun, 4, "nsubj", ""),
			tok(2, "do", "do", UniversalPos::Auxiliary, 4, "aux", "Tense=Pres"),
			tok(3, "n't", "not", UniversalPos::Particle, 2, "advmod", "Polarity=Neg"),
			tok(4, "know", "know", UniversalPos::Verb, 0, "root", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe()?)?;
		let clause = first_clause(&utterance)?;
		assert_eq!(clause.polarity, Polarity::Negative);
		Ok(())
	}

	#[test]
	fn i_do_know_stays_affirmative() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "I", "I", UniversalPos::Pronoun, 3, "nsubj", ""),
			tok(2, "do", "do", UniversalPos::Auxiliary, 3, "aux", "Tense=Pres"),
			tok(3, "know", "know", UniversalPos::Verb, 0, "root", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe()?)?;
		let clause = first_clause(&utterance)?;
		assert_eq!(clause.polarity, Polarity::Affirmative);
		Ok(())
	}

	#[test]
	fn live_udpipe_dont_know_is_negative() -> Result<(), LanguageError> {
		let Ok(parser) = crate::udpipe::UdpipeEnglishParser::bundled() else {
			return Ok(());
		};
		let marshaller = EnglishSemanticMarshaller::default();
		let negative = marshaller.marshal(
			&crate::parse::EnglishDependencyParser::parse(&parser, "I don't know.")?,
			&universe()?,
		)?;
		let affirmative = marshaller.marshal(
			&crate::parse::EnglishDependencyParser::parse(&parser, "I do know.")?,
			&universe()?,
		)?;
		assert_eq!(first_clause(&negative)?.polarity, Polarity::Negative);
		assert_eq!(first_clause(&affirmative)?.polarity, Polarity::Affirmative);
		Ok(())
	}

	#[test]
	fn i_am_going_for_a_walk() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "I", "I", UniversalPos::Pronoun, 3, "nsubj", ""),
			tok(2, "am", "be", UniversalPos::Auxiliary, 3, "aux", "Tense=Pres"),
			tok(3, "going", "go", UniversalPos::Verb, 0, "root", "VerbForm=Ger"),
			tok(4, "for", "for", UniversalPos::Adposition, 6, "case", ""),
			tok(5, "a", "a", UniversalPos::Determiner, 6, "det", ""),
			tok(6, "walk", "walk", UniversalPos::Noun, 3, "obl", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe()?)?;
		assert!(has_role(&utterance, SemanticRole::Agent));
		assert!(has_role(&utterance, SemanticRole::Goal));
		let clause = first_clause(&utterance)?;
		assert_eq!(clause.aspect, Aspect::Progressive);
		Ok(())
	}

	#[test]
	fn live_udpipe_i_am_going_for_a_walk() -> Result<(), LanguageError> {
		let Ok(parser) = crate::udpipe::UdpipeEnglishParser::bundled() else {
			return Ok(());
		};
		let document =
			crate::parse::EnglishDependencyParser::parse(&parser, "I am going for a walk.")?;
		let utterance = EnglishSemanticMarshaller::default().marshal(&document, &universe()?)?;
		assert!(!utterance.roots.is_empty());
		assert!(has_role(&utterance, SemanticRole::Agent));
		Ok(())
	}

	#[test]
	fn old_woman_gave_three_books_to_the_child() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "The", "the", UniversalPos::Determiner, 3, "det", ""),
			tok(2, "old", "old", UniversalPos::Adjective, 3, "amod", ""),
			tok(3, "woman", "woman", UniversalPos::Noun, 4, "nsubj", ""),
			tok(4, "gave", "give", UniversalPos::Verb, 0, "root", "Tense=Past"),
			tok(5, "three", "three", UniversalPos::Numeral, 6, "nummod", "NumType=Card"),
			tok(6, "books", "book", UniversalPos::Noun, 4, "obj", "Number=Plur"),
			tok(7, "to", "to", UniversalPos::Adposition, 9, "case", ""),
			tok(8, "the", "the", UniversalPos::Determiner, 9, "det", ""),
			tok(9, "child", "child", UniversalPos::Noun, 4, "obl", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe()?)?;
		assert!(utterance.referents.values().any(|referent| !referent.modifiers.is_empty()));
		assert!(utterance.referents.values().any(|referent| referent.number == Number::Plural));
		assert!(has_role(&utterance, SemanticRole::Recipient));
		Ok(())
	}

	#[test]
	fn we_all_walk_together() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "We", "we", UniversalPos::Pronoun, 3, "nsubj", "Number=Plur"),
			tok(2, "all", "all", UniversalPos::Determiner, 1, "det", ""),
			tok(3, "walk", "walk", UniversalPos::Verb, 0, "root", "Tense=Pres"),
			tok(4, "together", "together", UniversalPos::Adverb, 3, "advmod", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe()?)?;
		assert!(has_role(&utterance, SemanticRole::Agent));
		assert!(has_role(&utterance, SemanticRole::Manner));
		assert!(utterance.referents.values().any(|referent| referent.number == Number::Many));
		Ok(())
	}

	#[test]
	fn we_all_walk_together_to_the_river() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "We", "we", UniversalPos::Pronoun, 3, "nsubj", "Number=Plur"),
			tok(2, "all", "all", UniversalPos::Determiner, 1, "det", ""),
			tok(3, "walk", "walk", UniversalPos::Verb, 0, "root", "Tense=Pres"),
			tok(4, "together", "together", UniversalPos::Adverb, 3, "advmod", ""),
			tok(5, "to", "to", UniversalPos::Adposition, 7, "case", ""),
			tok(6, "the", "the", UniversalPos::Determiner, 7, "det", ""),
			tok(7, "river", "river", UniversalPos::Noun, 3, "obl", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe()?)?;
		assert!(has_role(&utterance, SemanticRole::Manner));
		assert!(has_role(&utterance, SemanticRole::Goal));
		Ok(())
	}

	#[test]
	fn we_all_walk_at_different_speeds_to_the_river() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "We", "we", UniversalPos::Pronoun, 3, "nsubj", "Number=Plur"),
			tok(2, "all", "all", UniversalPos::Adverb, 3, "advmod", ""),
			tok(3, "walk", "walk", UniversalPos::Verb, 0, "root", "Tense=Pres"),
			tok(4, "at", "at", UniversalPos::Adposition, 6, "case", ""),
			tok(5, "different", "different", UniversalPos::Adjective, 6, "amod", ""),
			tok(6, "speeds", "speed", UniversalPos::Noun, 3, "obl", "Number=Plur"),
			tok(7, "to", "to", UniversalPos::Adposition, 9, "case", ""),
			tok(8, "the", "the", UniversalPos::Determiner, 9, "det", ""),
			tok(9, "river", "river", UniversalPos::Noun, 3, "obl", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe()?)?;
		assert!(has_role(&utterance, SemanticRole::Agent));
		assert!(has_role(&utterance, SemanticRole::Rate));
		assert!(has_role(&utterance, SemanticRole::Goal));
		assert!(utterance.referents.values().any(|referent| !referent.modifiers.is_empty()));
		assert!(utterance.referents.values().any(|referent| referent.number == Number::Many));
		Ok(())
	}

	#[test]
	fn unknown_oblique_does_not_drop_the_clause() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "We", "we", UniversalPos::Pronoun, 2, "nsubj", ""),
			tok(2, "walk", "walk", UniversalPos::Verb, 0, "root", ""),
			tok(3, "despite", "despite", UniversalPos::Adposition, 4, "case", ""),
			tok(4, "rain", "rain", UniversalPos::Noun, 2, "obl", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe()?)?;
		assert_eq!(utterance.roots.len(), 1);
		assert!(has_role(&utterance, SemanticRole::Agent));
		assert!(!has_role(&utterance, SemanticRole::Goal));
		Ok(())
	}

	#[test]
	fn at_the_river_is_location_not_rate() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "We", "we", UniversalPos::Pronoun, 2, "nsubj", ""),
			tok(2, "walk", "walk", UniversalPos::Verb, 0, "root", ""),
			tok(3, "at", "at", UniversalPos::Adposition, 5, "case", ""),
			tok(4, "the", "the", UniversalPos::Determiner, 5, "det", ""),
			tok(5, "river", "river", UniversalPos::Noun, 2, "obl", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe()?)?;
		assert!(has_role(&utterance, SemanticRole::Location));
		assert!(!has_role(&utterance, SemanticRole::Rate));
		Ok(())
	}

	#[test]
	fn noun_root_walk_uses_nmod_obliques() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "We", "we", UniversalPos::Pronoun, 3, "nsubj", "Number=Plur"),
			tok(2, "all", "all", UniversalPos::Determiner, 3, "det", ""),
			tok(3, "walk", "walk", UniversalPos::Noun, 0, "root", ""),
			tok(4, "at", "at", UniversalPos::Adposition, 6, "case", ""),
			tok(5, "different", "different", UniversalPos::Adjective, 6, "amod", ""),
			tok(6, "speeds", "speed", UniversalPos::Noun, 3, "nmod", "Number=Plur"),
			tok(7, "to", "to", UniversalPos::Adposition, 9, "case", ""),
			tok(8, "the", "the", UniversalPos::Determiner, 9, "det", ""),
			tok(9, "river", "river", UniversalPos::Noun, 3, "nmod", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe()?)?;
		assert_eq!(utterance.roots.len(), 1);
		assert!(has_role(&utterance, SemanticRole::Agent));
		assert!(has_role(&utterance, SemanticRole::Rate));
		assert!(has_role(&utterance, SemanticRole::Goal));
		Ok(())
	}

	#[test]
	fn live_udpipe_when_the_bell_strikes_twelve() -> Result<(), LanguageError> {
		let Ok(parser) = crate::udpipe::UdpipeEnglishParser::bundled() else {
			return Ok(());
		};
		let document = crate::parse::EnglishDependencyParser::parse(
			&parser,
			"When the bell strikes 12, go to the river.",
		)?;
		let utterance =
			EnglishSemanticMarshaller::default().marshal(&document, &universe_for(&document)?)?;
		assert!(!utterance.roots.is_empty(), "empty utterance from:\n{}", document.debug_report());
		Ok(())
	}

	#[test]
	fn live_udpipe_walk_at_different_speeds() -> Result<(), LanguageError> {
		let Ok(parser) = crate::udpipe::UdpipeEnglishParser::bundled() else {
			return Ok(());
		};
		let document = crate::parse::EnglishDependencyParser::parse(
			&parser,
			"We all walk at different speeds to the river.",
		)?;
		assert!(
			!document.sentences.is_empty(),
			"UDPipe produced no sentences:\n{}",
			document.debug_report()
		);
		let utterance = EnglishSemanticMarshaller::default().marshal(&document, &universe()?)?;
		assert!(
			!utterance.roots.is_empty(),
			"empty utterance from:\n{}\n{utterance:#?}",
			document.debug_report()
		);
		assert!(has_role(&utterance, SemanticRole::Agent));
		assert!(has_role(&utterance, SemanticRole::Goal));
		assert!(
			has_role(&utterance, SemanticRole::Rate) || has_role(&utterance, SemanticRole::Manner)
		);
		Ok(())
	}

	#[test]
	fn mary_is_a_witch_is_classification() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "Mary", "Mary", UniversalPos::ProperNoun, 4, "nsubj", ""),
			tok(2, "is", "be", UniversalPos::Auxiliary, 4, "cop", "Tense=Pres"),
			tok(3, "a", "a", UniversalPos::Determiner, 4, "det", ""),
			tok(4, "witch", "witch", UniversalPos::Noun, 0, "root", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe()?)?;
		let clause = first_clause(&utterance)?;
		assert!(has_role(&utterance, SemanticRole::Theme));
		assert!(has_role(&utterance, SemanticRole::Classification));
		assert!(!has_role(&utterance, SemanticRole::Agent));
		assert_eq!(clause.arguments.len(), 2);
		Ok(())
	}

	#[test]
	fn he_saw_mary_resolves_third_person() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "He", "he", UniversalPos::Pronoun, 2, "nsubj", ""),
			tok(2, "saw", "see", UniversalPos::Verb, 0, "root", "Tense=Past"),
			tok(3, "Mary", "Mary", UniversalPos::ProperNoun, 2, "obj", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe()?)?;
		assert!(has_role(&utterance, SemanticRole::Experiencer));
		assert!(has_role(&utterance, SemanticRole::Theme));
		Ok(())
	}

	#[test]
	fn bob_is_overlaid_from_the_parse() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "Bob", "Bob", UniversalPos::ProperNoun, 2, "nsubj", ""),
			tok(2, "saw", "see", UniversalPos::Verb, 0, "root", "Tense=Past"),
			tok(3, "Mary", "Mary", UniversalPos::ProperNoun, 2, "obj", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe_for(&doc)?)?;
		assert!(has_role(&utterance, SemanticRole::Experiencer));
		assert!(has_role(&utterance, SemanticRole::Theme));
		Ok(())
	}

	#[test]
	fn when_the_bell_strikes_12_attaches_time() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "When", "when", UniversalPos::SubordinatingConjunction, 4, "mark", ""),
			tok(2, "the", "the", UniversalPos::Determiner, 3, "det", ""),
			tok(3, "bell", "bell", UniversalPos::Noun, 4, "nsubj", ""),
			tok(4, "strikes", "strike", UniversalPos::Verb, 6, "advcl", "Tense=Pres"),
			tok(5, "12", "12", UniversalPos::Numeral, 4, "obj", "NumType=Card"),
			tok(6, "go", "go", UniversalPos::Verb, 0, "root", "Mood=Imp"),
			tok(7, "to", "to", UniversalPos::Adposition, 9, "case", ""),
			tok(8, "the", "the", UniversalPos::Determiner, 9, "det", ""),
			tok(9, "river", "river", UniversalPos::Noun, 6, "obl", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe()?)?;
		assert_eq!(utterance.clauses.len(), 2);
		assert!(has_role(&utterance, SemanticRole::Time));
		assert!(has_role(&utterance, SemanticRole::Theme));
		assert!(has_role(&utterance, SemanticRole::Goal));
		Ok(())
	}

	#[test]
	fn all_that_was_given_binds_that_to_all() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "All", "all", UniversalPos::Determiner, 4, "nsubj:pass", ""),
			tok(2, "that", "that", UniversalPos::Pronoun, 4, "nsubj:pass", ""),
			tok(3, "was", "be", UniversalPos::Auxiliary, 4, "aux:pass", "Tense=Past"),
			tok(4, "given", "give", UniversalPos::Verb, 0, "root", "Voice=Pass"),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe()?)?;
		assert_eq!(utterance.referents.len(), 1);
		assert!(has_role(&utterance, SemanticRole::Theme));
		assert_eq!(first_clause(&utterance)?.arguments.len(), 1);
		Ok(())
	}

	#[test]
	fn all_that_was_given_is_gone_keeps_the_relative() -> Result<(), LanguageError> {
		let doc = sentence(vec![
			tok(1, "All", "all", UniversalPos::Determiner, 6, "nsubj", ""),
			tok(2, "that", "that", UniversalPos::Pronoun, 4, "nsubj:pass", ""),
			tok(3, "was", "be", UniversalPos::Auxiliary, 4, "aux:pass", "Tense=Past"),
			tok(4, "given", "give", UniversalPos::Verb, 1, "acl:relcl", "Voice=Pass"),
			tok(5, "is", "be", UniversalPos::Auxiliary, 6, "cop", "Tense=Pres"),
			tok(6, "gone", "gone", UniversalPos::Adjective, 0, "root", ""),
		]);
		let utterance = EnglishSemanticMarshaller::default().marshal(&doc, &universe()?)?;
		assert!(has_role(&utterance, SemanticRole::Theme));
		assert!(has_role(&utterance, SemanticRole::Classification));
		assert!(utterance
			.referents
			.values()
			.any(|referent| !referent.relative_clauses.is_empty()));
		assert_eq!(utterance.clauses.len(), 2);
		Ok(())
	}

	#[test]
	fn live_udpipe_all_that_was_given() -> Result<(), LanguageError> {
		let Ok(parser) = crate::udpipe::UdpipeEnglishParser::bundled() else {
			return Ok(());
		};
		let marshaller = EnglishSemanticMarshaller::default();
		for text in ["All that was given", "All that was given is gone."] {
			let document = crate::parse::EnglishDependencyParser::parse(&parser, text)?;
			let utterance = marshaller.marshal(&document, &universe_for(&document)?)?;
			assert!(
				!utterance.roots.is_empty(),
				"{text} produced no roots:\n{}",
				document.debug_report()
			);
			assert!(
				has_role(&utterance, SemanticRole::Theme)
					|| has_role(&utterance, SemanticRole::Classification),
				"{text} dropped its arguments:\n{}\n{utterance:#?}",
				document.debug_report()
			);
		}
		Ok(())
	}
}
