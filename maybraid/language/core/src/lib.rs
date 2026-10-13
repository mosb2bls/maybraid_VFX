//! Maybraid Language Core
//!
//! Base lexicon layer for procedurally lexicalizing structured semantics.
//!
//! ```text
//! Utterance -> Concepts -> Lexical Context -> Candidates -> Base Terms -> LexicalOutput
//! ```
//!
//! This crate implements the [semantic utterance → procedural lexicon
//! POC](https://github.com/ramate-io/maybraid/issues/897). [`Grammar`] realizes
//! an utterance plus lexicalizations into a structured surface IR.
//! [`SurfaceGrammar`] is the basic POC linearizer; typological composers live
//! in `maybraid-grammars`.
//!
//! WordNet 3.1 is the initial English concept universe. The full dictionary
//! lives under `assets/language/wordnet`; [`wordnet::WORDNET_LICENSE`]
//! reproduces Princeton's license as required.
//! Princeton University does not endorse this project. See
//! [`wordnet::WORDNET_CITATION`].

pub mod concept;
pub mod error;
pub mod grammar;
pub mod graph;
pub mod lexicalizer;
pub mod marshall;
pub mod output;
pub mod parse;
pub mod poc;
pub mod profile;
pub mod semantic;
pub mod term;
pub mod udpipe;
pub mod utterance;
pub mod wordnet;

pub use concept::{
	Concept, ConceptId, ConceptRelation, ConceptUniverse, EnglishSenseLookup, NeighborhoodRequest,
	Pos, RelationKind,
};
pub use error::{LanguageError, LanguagePipelineError};
pub use grammar::{
	AffixPlacement, BoundaryKind, ClauseOrder, Grammar, GrammarInput, GrammaticalOutput,
	GrammaticalRelation, IpaUtterance, LexicalPart, ModifierPlacement, ParticleDomain,
	RelativePlacement, RoleParticles, SurfaceClause, SurfaceConstituent, SurfaceForm,
	SurfaceGrammar,
};
pub use graph::{
	ConceptEdge, GraphDelta, InMemoryLexicalGraph, LexicalContextGraph, LexicalContextGraphMut,
	LexicalEdge, LexicalUpdate,
};
pub use lexicalizer::{
	CandidateContribution, CompositionalLexicalizer, LexicalCandidate, LexicalRealization,
	Lexicalizer, RootHeavyLexicalizer, GENERATOR_VERSION,
};
pub use marshall::{ConceptMarshaller, ConceptUse, DefaultMarshaller, SemanticNode};
pub use output::{LexicalOutput, ResolvedConcept};
pub use parse::{
	DependencyDocument, DependencyRelation, DependencySentence, DependencyToken,
	EnglishDependencyParser, MorphFeatures, TokenId, UniversalPos,
};
pub use poc::{poc_universe, PocLexicon};
pub use profile::{Profile, Register};
pub use semantic::{
	EnglishSemanticMarshaller, PredicateFrame, PredicateFrameLexicon, RoleMapping,
	SemanticMarshaller,
};
pub use term::{Ipa, Term, TermId, Usage};
pub use udpipe::{bundled_udpipe_path, UdpipeEnglishParser, BUNDLED_UDPIPE_FILE};
pub use utterance::{
	Argument, Aspect, Clause, ClauseId, Definiteness, FocusTarget, InformationStructure, Modifier,
	Mood, Number, Person, Polarity, Referent, ReferentId, SemanticRole, SemanticValue, Tense,
	Utterance,
};
pub use wordnet::{WordNetConceptUniverse, WORDNET_CITATION, WORDNET_LICENSE};

#[cfg(test)]
mod tests;
