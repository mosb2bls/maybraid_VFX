//! Convert [`GeneratedUtterance`] into a SlotMap semantic graph.

use std::collections::HashMap;

use maybraid_language_core::{
	Argument, Aspect, Clause, ConceptId, ConceptUniverse, Definiteness, Modifier, Mood, Number,
	Polarity, Pos, Referent, ReferentId, SemanticRole, SemanticValue, Tense, Utterance,
};

use crate::error::MistralLanguageError;
use crate::schema::{GeneratedArgument, GeneratedUtterance};

impl GeneratedUtterance {
	/// Build an utterance whose concepts are overlay IDs hashed from English terms.
	pub fn into_overlay_utterance(&self) -> Result<Utterance, MistralLanguageError> {
		self.into_utterance(&OverlayConcepts)
	}

	/// Resolve English concept labels through `universe`, then build the graph.
	pub fn into_utterance(
		&self,
		universe: &impl ConceptUniverse,
	) -> Result<Utterance, MistralLanguageError> {
		let mut utterance = Utterance::new();
		let mut referent_ids = HashMap::new();
		for referent in &self.referents {
			if referent_ids.contains_key(&referent.id) {
				return Err(MistralLanguageError::invalid_output(format!(
					"duplicate referent id {}",
					referent.id
				)));
			}
			let concept = resolve_concept(universe, &referent.concept, Some(Pos::Noun))?;
			let mut built = Referent::new(concept);
			built.definiteness = parse_definiteness(referent.definiteness.as_deref())?;
			built.number = parse_number(referent.number.as_deref())?;
			for modifier in &referent.modifiers {
				built.modifiers.push(Modifier {
					concept: resolve_concept(universe, modifier, Some(Pos::Adjective))?,
				});
			}
			referent_ids.insert(referent.id, utterance.add_referent(built));
		}

		let mut clause_ids = Vec::with_capacity(self.clauses.len());
		for clause in &self.clauses {
			let predicate = resolve_concept(universe, &clause.predicate, Some(Pos::Verb))?;
			clause_ids.push(utterance.add_clause(Clause::new(predicate)));
		}

		for (index, generated) in self.clauses.iter().enumerate() {
			let Some(clause) = utterance.clauses.get_mut(clause_ids[index]) else {
				return Err(MistralLanguageError::invalid_output(format!(
					"missing clause slot {index}"
				)));
			};
			clause.tense = parse_tense(generated.tense.as_deref())?;
			clause.aspect = parse_aspect(generated.aspect.as_deref())?;
			clause.mood = parse_mood(generated.mood.as_deref())?;
			clause.polarity = parse_polarity(generated.polarity.as_deref())?;
			for argument in &generated.arguments {
				let Some(repaired) = argument.repaired() else {
					if is_agent_role(&argument.role) {
						if let Some(agent) = default_agent(&self.referents, &referent_ids) {
							clause.arguments.push(Argument {
								role: SemanticRole::Agent,
								value: SemanticValue::Referent(agent),
							});
						}
					}
					continue;
				};
				match repaired.to_semantic(&referent_ids, &clause_ids) {
					Ok(Some(argument)) => clause.arguments.push(argument),
					Ok(None) => {}
					Err(error) => return Err(error),
				}
			}
		}

		for referent in &self.referents {
			let Some(&id) = referent_ids.get(&referent.id) else {
				continue;
			};
			for relative in &referent.relative_clauses {
				let Some(clause) = clause_at(&clause_ids, *relative) else {
					continue;
				};
				if let Some(stored) = utterance.referents.get_mut(id) {
					stored.relative_clauses.push(clause);
				}
			}
		}

		let mut any_root = false;
		for root in &self.roots {
			if let Some(clause) = clause_at(&clause_ids, *root) {
				utterance.push_root(clause);
				any_root = true;
			}
		}
		if !any_root {
			for clause in clause_ids {
				utterance.push_root(clause);
			}
		}
		Ok(utterance)
	}
}

impl GeneratedArgument {
	/// Drop empty role-only args; if both targets are set, keep the referent.
	fn repaired(&self) -> Option<Self> {
		match (self.referent, self.clause) {
			(None, None) => None,
			(Some(_), Some(_)) => Some(Self { clause: None, ..self.clone() }),
			_ => Some(self.clone()),
		}
	}

	fn to_semantic(
		&self,
		referents: &HashMap<usize, ReferentId>,
		clauses: &[maybraid_language_core::ClauseId],
	) -> Result<Option<Argument>, MistralLanguageError> {
		let role = parse_role(&self.role)?;
		let value = match (self.referent, self.clause) {
			(Some(referent), None) => {
				let Some(id) = referents.get(&referent).copied() else {
					return Ok(None);
				};
				SemanticValue::Referent(id)
			}
			(None, Some(clause)) => {
				let Some(id) = clause_at(clauses, clause) else {
					return Ok(None);
				};
				SemanticValue::Clause(id)
			}
			_ => return Ok(None),
		};
		Ok(Some(Argument { role, value }))
	}
}

fn default_agent(
	referents: &[crate::schema::GeneratedReferent],
	ids: &HashMap<usize, ReferentId>,
) -> Option<ReferentId> {
	let people: Vec<ReferentId> = referents
		.iter()
		.filter(|referent| is_person_like(&referent.concept))
		.filter_map(|referent| ids.get(&referent.id).copied())
		.collect();
	if people.len() == 1 {
		return people.first().copied();
	}
	if ids.len() == 1 {
		return ids.values().next().copied();
	}
	None
}

fn is_person_like(concept: &str) -> bool {
	aliases(concept)
		.iter()
		.any(|alias| matches!(alias.as_str(), "speaker" | "listener" | "person"))
}

fn is_agent_role(role: &str) -> bool {
	matches!(normalize_lemma(role).as_str(), "agent" | "subject")
}

fn clause_at(
	clauses: &[maybraid_language_core::ClauseId],
	index: usize,
) -> Option<maybraid_language_core::ClauseId> {
	clauses.get(index).copied()
}

fn resolve_concept(
	universe: &impl ConceptUniverse,
	name: &str,
	prefer: Option<Pos>,
) -> Result<ConceptId, MistralLanguageError> {
	for alias in aliases(name) {
		let ids = universe.resolve_english(&alias);
		if let Some(id) = pick_concept(&ids, prefer) {
			return Ok(id);
		}
	}
	Err(LanguageMissing(name.to_owned()).into())
}

struct LanguageMissing(String);

impl From<LanguageMissing> for MistralLanguageError {
	fn from(value: LanguageMissing) -> Self {
		MistralLanguageError::Language(maybraid_language_core::LanguageError::MissingConcept(
			value.0,
		))
	}
}

fn pick_concept(ids: &[ConceptId], prefer: Option<Pos>) -> Option<ConceptId> {
	if let Some(pos) = prefer {
		if let Some(id) = ids.iter().copied().find(|id| id.pos() == Some(pos)) {
			return Some(id);
		}
	}
	ids.first().copied()
}

const DETERMINERS: &[&str] = &[
	"a", "an", "the", "this", "that", "these", "those", "my", "your", "his", "her", "their", "our",
	"some", "any",
];

fn aliases(name: &str) -> Vec<String> {
	let normalized = normalize_lemma(name);
	let tokens: Vec<&str> = normalized.split('_').filter(|token| !token.is_empty()).collect();
	let stripped = strip_leading_determiners(&tokens);
	if stripped.len() <= 1 {
		if let Some(mapped) =
			pronoun_aliases(stripped.first().copied().unwrap_or(normalized.as_str()))
		{
			return mapped;
		}
	}
	if let Some(mapped) = pronoun_aliases(&normalized) {
		return mapped;
	}

	let stripped_joined = (!stripped.is_empty()).then(|| stripped.join("_"));
	let stripped_last = stripped.last().map(|last| (*last).to_owned());
	let mut out = Vec::new();
	push_unique(&mut out, normalized);
	if let Some(joined) = stripped_joined {
		push_unique(&mut out, joined);
	}
	if let Some(last) = stripped_last {
		push_unique(&mut out, last);
	}
	out
}

fn pronoun_aliases(lemma: &str) -> Option<Vec<String>> {
	match lemma {
		"i" | "me" | "we" | "us" | "myself" | "speaker" | "self" => {
			Some(vec!["speaker".to_owned(), "person".to_owned()])
		}
		"you" | "listener" | "addressee" | "hearer" => {
			Some(vec!["listener".to_owned(), "person".to_owned()])
		}
		"he" | "him" | "his" | "she" | "her" | "they" | "them" | "their" | "himself"
		| "herself" | "themselves" | "person" => Some(vec!["person".to_owned()]),
		"it" | "its" | "itself" | "thing" => Some(vec!["thing".to_owned()]),
		_ => None,
	}
}

fn strip_leading_determiners<'a>(tokens: &[&'a str]) -> Vec<&'a str> {
	let mut start = 0;
	while start < tokens.len() && DETERMINERS.contains(&tokens[start]) {
		start += 1;
	}
	tokens[start..].to_vec()
}

fn push_unique(out: &mut Vec<String>, candidate: String) {
	if candidate.is_empty() || out.iter().any(|existing| existing == &candidate) {
		return;
	}
	out.push(candidate);
}

fn normalize_lemma(term: &str) -> String {
	let mut out = String::new();
	let mut pending_separator = false;
	for ch in term.trim().chars() {
		let ch = ch.to_ascii_lowercase();
		if ch.is_ascii_alphanumeric() {
			if pending_separator && !out.is_empty() {
				out.push('_');
			}
			out.push(ch);
			pending_separator = false;
		} else if matches!(ch, ' ' | '_' | '-' | '\'') || ch.is_ascii_whitespace() {
			pending_separator = true;
		}
	}
	out
}

fn parse_role(name: &str) -> Result<SemanticRole, MistralLanguageError> {
	match normalize_lemma(name).as_str() {
		"agent" | "subject" => Ok(SemanticRole::Agent),
		"patient" | "object" => Ok(SemanticRole::Patient),
		"theme" => Ok(SemanticRole::Theme),
		"experiencer" => Ok(SemanticRole::Experiencer),
		"recipient" | "iobj" => Ok(SemanticRole::Recipient),
		"beneficiary" => Ok(SemanticRole::Beneficiary),
		"instrument" => Ok(SemanticRole::Instrument),
		"location" | "loc" => Ok(SemanticRole::Location),
		"source" => Ok(SemanticRole::Source),
		"goal" => Ok(SemanticRole::Goal),
		"possessor" => Ok(SemanticRole::Possessor),
		"cause" => Ok(SemanticRole::Cause),
		"content" => Ok(SemanticRole::Content),
		"classification" | "class" => Ok(SemanticRole::Classification),
		"manner" => Ok(SemanticRole::Manner),
		"rate" => Ok(SemanticRole::Rate),
		"time" => Ok(SemanticRole::Time),
		other => Err(MistralLanguageError::invalid_output(format!("unknown role {other:?}"))),
	}
}

fn parse_tense(name: Option<&str>) -> Result<Tense, MistralLanguageError> {
	match name.map(normalize_lemma).as_deref() {
		None | Some("unspecified") => Ok(Tense::Unspecified),
		Some("past") => Ok(Tense::Past),
		Some("present") => Ok(Tense::Present),
		Some("future") => Ok(Tense::Future),
		Some(other) => {
			Err(MistralLanguageError::invalid_output(format!("unknown tense {other:?}")))
		}
	}
}

fn parse_aspect(name: Option<&str>) -> Result<Aspect, MistralLanguageError> {
	match name.map(normalize_lemma).as_deref() {
		None | Some("unspecified") => Ok(Aspect::Unspecified),
		Some("simple") => Ok(Aspect::Simple),
		Some("progressive") => Ok(Aspect::Progressive),
		Some("perfect") => Ok(Aspect::Perfect),
		Some(other) => {
			Err(MistralLanguageError::invalid_output(format!("unknown aspect {other:?}")))
		}
	}
}

fn parse_mood(name: Option<&str>) -> Result<Mood, MistralLanguageError> {
	match name.map(normalize_lemma).as_deref() {
		None | Some("unspecified") => Ok(Mood::Unspecified),
		Some("indicative") => Ok(Mood::Indicative),
		Some("imperative") => Ok(Mood::Imperative),
		Some("interrogative") => Ok(Mood::Interrogative),
		Some(other) => Err(MistralLanguageError::invalid_output(format!("unknown mood {other:?}"))),
	}
}

fn parse_polarity(name: Option<&str>) -> Result<Polarity, MistralLanguageError> {
	match name.map(normalize_lemma).as_deref() {
		None | Some("affirmative") => Ok(Polarity::Affirmative),
		Some("negative") => Ok(Polarity::Negative),
		Some(other) => {
			Err(MistralLanguageError::invalid_output(format!("unknown polarity {other:?}")))
		}
	}
}

fn parse_definiteness(name: Option<&str>) -> Result<Definiteness, MistralLanguageError> {
	match name.map(normalize_lemma).as_deref() {
		None | Some("indefinite") => Ok(Definiteness::Indefinite),
		Some("definite") => Ok(Definiteness::Definite),
		Some("generic") => Ok(Definiteness::Generic),
		Some("proper") => Ok(Definiteness::Proper),
		Some(other) => {
			Err(MistralLanguageError::invalid_output(format!("unknown definiteness {other:?}")))
		}
	}
}

fn parse_number(name: Option<&str>) -> Result<Number, MistralLanguageError> {
	match name.map(normalize_lemma).as_deref() {
		None | Some("singular") => Ok(Number::Singular),
		Some("plural") => Ok(Number::Plural),
		Some("many") => Ok(Number::Many),
		Some("mass") => Ok(Number::Mass),
		Some("unspecified") => Ok(Number::Unspecified),
		Some(other) => {
			Err(MistralLanguageError::invalid_output(format!("unknown number {other:?}")))
		}
	}
}

/// Fallback universe: each English label becomes a stable overlay concept.
struct OverlayConcepts;

impl ConceptUniverse for OverlayConcepts {
	fn concept(&self, id: ConceptId) -> Option<maybraid_language_core::Concept> {
		Some(maybraid_language_core::Concept { id, english_glosses: Vec::new() })
	}

	fn resolve_english(&self, term: &str) -> Vec<ConceptId> {
		vec![ConceptId::overlay(overlay_tag(&normalize_lemma(term)))]
	}

	fn neighborhood(
		&self,
		_concept: ConceptId,
		_request: maybraid_language_core::NeighborhoodRequest,
	) -> Vec<maybraid_language_core::ConceptRelation> {
		Vec::new()
	}
}

fn overlay_tag(lemma: &str) -> u32 {
	let mut hash = 2166136261_u32;
	for byte in lemma.as_bytes() {
		hash ^= u32::from(*byte);
		hash = hash.wrapping_mul(16777619);
	}
	hash & 0x00FF_FFFF
}
