//! Orchestrate English → (optional Qwen) → UDPipe → utterance → POC language.

use std::path::PathBuf;

use anyhow::bail;
use clap::Parser;
use maybraid_language_core::{
	poc_universe, CompositionalLexicalizer, ConceptUniverse, DependencyDocument,
	EnglishDependencyParser, EnglishSemanticMarshaller, InMemoryLexicalGraph,
	LanguagePipelineError, LexicalOutput, Profile, RootHeavyLexicalizer, SemanticMarshaller,
	SurfaceGrammar, UdpipeEnglishParser, Utterance, WordNetConceptUniverse,
};
use maybraid_language_mistral::{
	bundled_model_path, MistralLanguageConfig, MistralLanguageModel, ResponseRequest,
};

#[derive(Parser, Debug)]
#[command(name = "maybraid-language", about = "Translate or respond through a generated language")]
struct Args {
	#[arg(long, conflicts_with = "respond")]
	translate: Option<String>,

	#[arg(long, alias = "repond", conflicts_with = "translate")]
	respond: Option<String>,

	#[arg(long)]
	to_language_number: usize,

	#[arg(long)]
	with_english: bool,

	#[arg(long)]
	debug_language: bool,

	#[arg(long)]
	model_path: Option<PathBuf>,

	#[arg(long)]
	udpipe_path: Option<PathBuf>,

	#[arg(long)]
	force_cpu: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let args = Args::parse();
	let input = match (&args.translate, &args.respond) {
		(Some(text), None) | (None, Some(text)) => text.as_str(),
		_ => bail!("specify exactly one of --translate or --respond"),
	};
	let language = PocLanguage::from_number(args.to_language_number)?;
	let parser = match &args.udpipe_path {
		Some(path) => UdpipeEnglishParser::from_path(path).map_err(LanguagePipelineError::from)?,
		None => UdpipeEnglishParser::bundled().map_err(LanguagePipelineError::from)?,
	};
	let marshaller = EnglishSemanticMarshaller::default();

	let (english, source, parsed, utterance, universe) = if args.respond.is_some() {
		let model_path = args.model_path.clone().unwrap_or_else(bundled_model_path);
		let mut config = MistralLanguageConfig::from_path(model_path);
		if args.force_cpu {
			config = config.with_force_cpu(true);
		}
		let model = MistralLanguageModel::load(config)
			.await
			.map_err(|error| LanguagePipelineError::ResponseGeneration(error.to_string()))?;
		let english = model
			.respond(ResponseRequest::new(input))
			.await
			.map_err(|error| LanguagePipelineError::ResponseGeneration(error.to_string()))?;
		let parsed = parser.parse(&english).map_err(LanguagePipelineError::from)?;
		let universe = poc_universe_for(&parsed)?;
		let utterance =
			marshaller.marshal(&parsed, &universe).map_err(LanguagePipelineError::from)?;
		(Some(english), input.to_owned(), parsed, utterance, universe)
	} else {
		let parsed = parser.parse(input).map_err(LanguagePipelineError::from)?;
		let universe = poc_universe_for(&parsed)?;
		let utterance =
			marshaller.marshal(&parsed, &universe).map_err(LanguagePipelineError::from)?;
		(None, input.to_owned(), parsed, utterance, universe)
	};

	let rendered = language.render(utterance.clone(), &universe);
	if args.debug_language {
		print_debug(
			&source,
			english.as_deref(),
			&parsed,
			&utterance,
			&rendered,
			&universe,
			&language,
		);
	} else if args.with_english {
		println!("English:\n{}\n", display_english(english.as_deref(), &source));
		println!("Language {}:\n{}", language.number, rendered.ipa);
	} else {
		println!("{}", rendered.ipa);
	}
	Ok(())
}

struct PocLanguage {
	number: usize,
	kind: PocLanguageKind,
	seed: u64,
}

enum PocLanguageKind {
	Compositional,
	RootHeavy,
}

struct RenderedLanguage {
	ipa: String,
	lexical: LexicalOutput,
	graph: InMemoryLexicalGraph,
}

impl PocLanguage {
	fn from_number(number: usize) -> anyhow::Result<Self> {
		let language = match number {
			0 => Self {
				number,
				kind: PocLanguageKind::Compositional,
				seed: CompositionalLexicalizer::DEFAULT_SEED,
			},
			1 => Self {
				number,
				kind: PocLanguageKind::RootHeavy,
				seed: RootHeavyLexicalizer::DEFAULT_SEED,
			},
			2 => Self { number, kind: PocLanguageKind::Compositional, seed: 0xC33C_C33C_C33C_C33C },
			3 => Self { number, kind: PocLanguageKind::RootHeavy, seed: 0xD44D_D44D_D44D_D44D },
			_ => bail!("unsupported --to-language-number {number}; use 0..=3"),
		};
		Ok(language)
	}

	fn grammar(&self) -> SurfaceGrammar {
		match self.kind {
			PocLanguageKind::Compositional => SurfaceGrammar::compositional(),
			PocLanguageKind::RootHeavy => SurfaceGrammar::root_heavy(),
		}
	}

	fn render(&self, utterance: Utterance, universe: &WordNetConceptUniverse) -> RenderedLanguage {
		let mut graph = InMemoryLexicalGraph::new();
		let profile = Profile::neutral();
		let lexical = match self.kind {
			PocLanguageKind::Compositional => LexicalOutput::render(
				utterance,
				&CompositionalLexicalizer::new(self.seed),
				universe,
				&mut graph,
				&profile,
			),
			PocLanguageKind::RootHeavy => LexicalOutput::render(
				utterance,
				&RootHeavyLexicalizer::new(self.seed),
				universe,
				&mut graph,
				&profile,
			),
		};
		let ipa = lexical.realize(self.grammar()).ipa().to_string();
		RenderedLanguage { ipa, lexical, graph }
	}
}

fn poc_universe_for(parsed: &DependencyDocument) -> anyhow::Result<WordNetConceptUniverse> {
	Ok(poc_universe()?
		.with_proper_names(["John", "Mary", "Alice", "speaker", "listener"])
		.with_document_overlays(parsed))
}

fn display_english<'a>(generated: Option<&'a str>, source: &'a str) -> &'a str {
	generated.unwrap_or(source)
}

fn print_debug(
	input: &str,
	english: Option<&str>,
	parsed: &DependencyDocument,
	utterance: &Utterance,
	rendered: &RenderedLanguage,
	universe: &WordNetConceptUniverse,
	language: &PocLanguage,
) {
	println!("Input:\n{input}\n");
	if let Some(english) = english {
		println!("Generated English:\n{english}\n");
	}
	println!("UDPipe:\n{}", parsed.debug_report());
	println!("Utterance:\n{utterance:#?}\n");
	println!("Resolved concepts:");
	for referent in utterance.referents.values() {
		let label = universe
			.concept(referent.concept)
			.map(|concept| concept.primary_gloss().to_owned())
			.unwrap_or_else(|| referent.concept.to_string());
		println!("  {label} ({})", referent.concept);
	}
	for clause in utterance.clauses.values() {
		let label = universe
			.concept(clause.predicate)
			.map(|concept| concept.primary_gloss().to_owned())
			.unwrap_or_else(|| clause.predicate.to_string());
		println!("  {label} ({})", clause.predicate);
	}
	println!();
	print!("{}", rendered.lexical.debug_report("Lexicalizations:", universe, &rendered.graph));
	println!("Target output:\n{}", rendered.ipa);
	println!("\nLanguage number: {}", language.number);
}

#[cfg(test)]
mod tests {
	use clap::Parser;
	use maybraid_language_core::CompositionalLexicalizer;

	use super::{Args, PocLanguage};

	#[test]
	fn translate_args_parse() -> anyhow::Result<()> {
		let args = Args::try_parse_from([
			"maybraid-language",
			"--translate",
			"I am going for a walk.",
			"--to-language-number",
			"0",
		])
		.map_err(|error| anyhow::anyhow!("{error}"))?;
		assert_eq!(args.translate.as_deref(), Some("I am going for a walk."));
		assert_eq!(args.to_language_number, 0);
		Ok(())
	}

	#[test]
	fn respond_alias_repond_is_accepted() -> anyhow::Result<()> {
		let args = Args::try_parse_from([
			"maybraid-language",
			"--repond",
			"I am going for a walk.",
			"--to-language-number",
			"3",
			"--with-english",
		])
		.map_err(|error| anyhow::anyhow!("{error}"))?;
		assert_eq!(args.respond.as_deref(), Some("I am going for a walk."));
		assert!(args.with_english);
		Ok(())
	}

	#[test]
	fn language_numbers_are_deterministic() -> anyhow::Result<()> {
		let zero = PocLanguage::from_number(0)?;
		let three = PocLanguage::from_number(3)?;
		assert_eq!(zero.seed, CompositionalLexicalizer::DEFAULT_SEED);
		assert_ne!(zero.seed, three.seed);
		Ok(())
	}

	#[test]
	fn with_english_falls_back_to_source_text() {
		assert_eq!(super::display_english(None, "I do know."), "I do know.");
		assert_eq!(super::display_english(Some("Sure."), "hello"), "Sure.");
	}
}
