//! Composable grammar strategies for Maybraid languages.
//!
//! [`maybraid_language_core::Grammar`] and the surface IR live in
//! `maybraid-language-core`. This crate supplies reusable composers and
//! typological presets as specified in
//! [issue #913](https://github.com/ramate-io/maybraid/issues/913).

pub mod agreement;
pub mod alignment;
pub mod clause;
pub mod composite;
pub mod copula;
pub mod information;
pub mod linking;
pub mod marking;
pub mod nominal;
pub mod predicate;
pub mod presets;
pub mod question;

pub use agreement::{AgreementController, AgreementFeatures, AgreementStrategy};
pub use alignment::{Alignment, CaseLabel};
pub use clause::WordOrder;
pub use composite::CompositeGrammar;
pub use copula::{CopulaStrategy, PredicationKind};
pub use information::InformationStrategy;
pub use linking::ClauseLinkStrategy;
pub use marking::{AdpositionStrategy, CaseStrategy, Marking, PolarityStrategy, TamStrategy};
pub use nominal::{DefinitenessStrategy, NumberStrategy};
pub use predicate::{PredicateStrategy, SerialStrategy};
pub use presets::{
	agglutinative_sov, basic_compositional, ergative_vso, fusional_svo, isolating_svo,
	particle_heavy_topic_prominent, separable_svo, serial_verb,
};
pub use question::QuestionStrategy;

impl CompositeGrammar {
	pub fn isolating_svo() -> Self {
		isolating_svo()
	}

	pub fn agglutinative_sov() -> Self {
		agglutinative_sov()
	}

	pub fn fusional_svo() -> Self {
		fusional_svo()
	}

	pub fn particle_heavy_topic_prominent() -> Self {
		particle_heavy_topic_prominent()
	}

	pub fn serial_verb() -> Self {
		serial_verb()
	}

	pub fn ergative_vso() -> Self {
		ergative_vso()
	}

	pub fn separable_svo() -> Self {
		separable_svo()
	}

	pub fn basic_compositional() -> Self {
		basic_compositional()
	}
}

pub use maybraid_language_core::{Grammar, GrammarInput, SurfaceForm, SurfaceGrammar};

#[cfg(test)]
mod tests;
