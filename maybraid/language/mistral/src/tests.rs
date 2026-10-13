use anyhow::Context;
use maybraid_language_core::ConceptUniverse;

use crate::config::{
	bundled_model_path, ParseGenerationConfig, ResponseGenerationConfig, BUNDLED_MODEL_FILE,
	LEGACY_MODEL_FILE, PARSE_MAX_LEN, RESPOND_MAX_LEN,
};
use crate::device::InferenceDevice;
use crate::prompt::RespondPrompt;
use crate::schema::{
	GeneratedClause, GeneratedReferent, GeneratedUtterance, MAX_CLAUSES, MAX_REFERENTS, MAX_ROOTS,
};

const ISSUE_EXAMPLE: &str = r#"
{
  "referents": [
    { "id": 0, "concept": "speaker" },
    { "id": 1, "concept": "walk" }
  ],
  "clauses": [
    {
      "predicate": "go",
      "arguments": [
        { "role": "Agent", "referent": 0 }
      ]
    }
  ],
  "roots": [0]
}
"#;

#[test]
fn bundled_model_path_points_at_the_shared_asset() {
	if std::env::var("MAYBRAID_QWEN_GGUF").is_ok() {
		return;
	}
	let path = bundled_model_path();
	assert!(
		path.ends_with(BUNDLED_MODEL_FILE) || path.ends_with(LEGACY_MODEL_FILE),
		"bundled path should end with {BUNDLED_MODEL_FILE} or {LEGACY_MODEL_FILE}, got {}",
		path.display()
	);
	assert!(
		path.to_string_lossy().contains("assets/models"),
		"bundled path should live under assets/models, got {}",
		path.display()
	);
}

#[test]
fn respond_prompt_includes_bounded_context() {
	let context = RespondPrompt::empty_context();
	let prompt = RespondPrompt::user("I am going for a walk.", &context);
	assert!(prompt.contains("I am going for a walk."));
	assert!(prompt.contains("nearby_entities"));
}

#[test]
fn parse_generation_defaults_are_deterministic() {
	let parse = ParseGenerationConfig::default();
	assert_eq!(parse.temperature, 0.0);
	assert_eq!(parse.max_len, PARSE_MAX_LEN);
	assert_eq!(ResponseGenerationConfig::default().max_len, RESPOND_MAX_LEN);
}

#[test]
fn force_cpu_selects_the_cpu_device() -> anyhow::Result<()> {
	let device = InferenceDevice::select(true)?;
	assert!(device.is_cpu());
	assert!(!InferenceDevice::is_gpu(&device));
	Ok(())
}

#[test]
fn generated_utterance_deserializes_the_issue_example() -> anyhow::Result<()> {
	let generated = GeneratedUtterance::from_json(ISSUE_EXAMPLE)?;
	assert_eq!(generated.referents.len(), 2);
	assert_eq!(generated.clauses.len(), 1);
	assert_eq!(generated.roots, vec![0]);
	assert_eq!(generated.referents[0].concept, "speaker");
	assert_eq!(generated.clauses[0].predicate, "go");
	Ok(())
}

#[test]
fn overlay_conversion_reuses_referent_ids() -> anyhow::Result<()> {
	let generated = GeneratedUtterance::from_json(ISSUE_EXAMPLE)?;
	let utterance = generated.into_overlay_utterance()?;
	assert_eq!(utterance.referents.len(), 2);
	assert_eq!(utterance.clauses.len(), 1);
	assert_eq!(utterance.roots.len(), 1);
	Ok(())
}

#[test]
fn wordnet_conversion_strips_articles_from_surface_nps() -> anyhow::Result<()> {
	let universe = maybraid_language_core::poc_universe()?
		.with_proper_names(["John", "Mary", "speaker", "listener"]);
	let generated = GeneratedUtterance {
		referents: vec![
			GeneratedReferent { id: 0, concept: "the speaker".to_owned(), ..Default::default() },
			GeneratedReferent { id: 1, concept: "a walk".to_owned(), ..Default::default() },
		],
		clauses: vec![GeneratedClause {
			predicate: "go".to_owned(),
			arguments: vec![crate::schema::GeneratedArgument {
				role: "Agent".to_owned(),
				referent: Some(0),
				clause: None,
			}],
			..Default::default()
		}],
		roots: vec![0],
	};
	let utterance = generated.into_utterance(&universe)?;
	let walk = utterance
		.referents
		.values()
		.find(|referent| {
			universe
				.concept(referent.concept)
				.is_some_and(|concept| concept.english_glosses.iter().any(|gloss| gloss == "walk"))
		})
		.context("a walk should resolve to WordNet walk")?;
	assert!(universe.concept(walk.concept).is_some());
	Ok(())
}

#[test]
fn wordnet_conversion_resolves_english_labels() -> anyhow::Result<()> {
	let universe = maybraid_language_core::poc_universe()?
		.with_proper_names(["John", "Mary", "speaker", "listener"]);
	let generated = GeneratedUtterance::from_json(ISSUE_EXAMPLE)?;
	let utterance = generated.into_utterance(&universe)?;
	let first = utterance.referents.values().next().context("expected a converted referent")?;
	assert!(
		universe.concept(first.concept).is_some(),
		"speaker should resolve in the concept universe"
	);
	let predicate = utterance.clauses.values().next().context("expected a converted clause")?;
	assert!(universe.concept(predicate.predicate).is_some(), "go should resolve through WordNet");
	Ok(())
}

#[test]
fn relative_clause_attaches_without_slotmap_ids() -> anyhow::Result<()> {
	let generated = GeneratedUtterance {
		referents: vec![
			GeneratedReferent {
				id: 0,
				concept: "mary".to_owned(),
				relative_clauses: vec![0],
				..Default::default()
			},
			GeneratedReferent { id: 1, concept: "witch".to_owned(), ..Default::default() },
		],
		clauses: vec![GeneratedClause {
			predicate: "be".to_owned(),
			arguments: vec![
				crate::schema::GeneratedArgument {
					role: "Theme".to_owned(),
					referent: Some(0),
					clause: None,
				},
				crate::schema::GeneratedArgument {
					role: "Classification".to_owned(),
					referent: Some(1),
					clause: None,
				},
			],
			..Default::default()
		}],
		roots: vec![0],
	};
	let utterance = generated.into_overlay_utterance()?;
	let mary = utterance.referents.values().next().context("mary")?;
	assert_eq!(mary.relative_clauses.len(), 1);
	Ok(())
}

#[test]
fn json_schema_is_an_object_constraint() {
	let schema = GeneratedUtterance::json_schema();
	assert_eq!(schema["type"], "object");
	assert_eq!(schema["required"][0], "referents");
	assert_eq!(
		schema["properties"]["referents"]["maxItems"].as_u64(),
		Some(u64::from(MAX_REFERENTS))
	);
	assert_eq!(schema["properties"]["clauses"]["maxItems"].as_u64(), Some(u64::from(MAX_CLAUSES)));
	assert_eq!(schema["properties"]["roots"]["maxItems"].as_u64(), Some(u64::from(MAX_ROOTS)));
	let argument = &schema["properties"]["clauses"]["items"]["properties"]["arguments"]["items"];
	assert_eq!(argument["oneOf"].as_array().map(Vec::len), Some(2));
	let roles = argument["oneOf"][0]["properties"]["role"]["enum"].as_array();
	assert!(roles.is_some_and(|roles| {
		roles.iter().any(|role| role == "Manner")
			&& roles.iter().any(|role| role == "Rate")
			&& roles.iter().any(|role| role == "Time")
	}));
}

#[test]
fn dangling_agent_attaches_the_speaker() -> anyhow::Result<()> {
	let generated = GeneratedUtterance {
		referents: vec![
			GeneratedReferent { id: 0, concept: "speaker".to_owned(), ..Default::default() },
			GeneratedReferent { id: 1, concept: "walk".to_owned(), ..Default::default() },
		],
		clauses: vec![GeneratedClause {
			predicate: "go".to_owned(),
			arguments: vec![crate::schema::GeneratedArgument {
				role: "Agent".to_owned(),
				referent: None,
				clause: None,
			}],
			..Default::default()
		}],
		roots: vec![0],
	};
	let utterance = generated.into_overlay_utterance()?;
	let clause = utterance.clauses.values().next().context("clause")?;
	assert_eq!(clause.arguments.len(), 1);
	assert_eq!(clause.arguments[0].role, maybraid_language_core::SemanticRole::Agent);
	Ok(())
}

#[test]
fn copular_clause_does_not_gain_a_default_agent() -> anyhow::Result<()> {
	let generated = GeneratedUtterance {
		referents: vec![
			GeneratedReferent { id: 0, concept: "mary".to_owned(), ..Default::default() },
			GeneratedReferent { id: 1, concept: "witch".to_owned(), ..Default::default() },
		],
		clauses: vec![GeneratedClause {
			predicate: "be".to_owned(),
			arguments: vec![
				crate::schema::GeneratedArgument {
					role: "Theme".to_owned(),
					referent: Some(0),
					clause: None,
				},
				crate::schema::GeneratedArgument {
					role: "Classification".to_owned(),
					referent: Some(1),
					clause: None,
				},
			],
			..Default::default()
		}],
		roots: vec![0],
	};
	let utterance = generated.into_overlay_utterance()?;
	let clause = utterance.clauses.values().next().context("clause")?;
	assert!(!clause
		.arguments
		.iter()
		.any(|argument| { argument.role == maybraid_language_core::SemanticRole::Agent }));
	assert_eq!(clause.arguments.len(), 2);
	Ok(())
}

#[test]
fn out_of_range_clause_indexes_are_dropped() -> anyhow::Result<()> {
	let generated = GeneratedUtterance::from_json(
		r#"{
		  "referents":[{"id":1,"concept":"walk","relative_clauses":[2]}],
		  "clauses":[{"predicate":"go","arguments":[{"role":"Classification","referent":1}]}],
		  "roots":[1]
		}"#,
	)?;
	let utterance = generated.into_overlay_utterance()?;
	assert_eq!(utterance.clauses.len(), 1);
	assert_eq!(utterance.roots.len(), 1);
	let walk = utterance.referents.values().next().context("walk")?;
	assert!(walk.relative_clauses.is_empty());
	Ok(())
}

#[test]
fn both_argument_targets_prefer_the_referent() -> anyhow::Result<()> {
	let generated = GeneratedUtterance {
		referents: vec![GeneratedReferent {
			id: 0,
			concept: "speaker".to_owned(),
			..Default::default()
		}],
		clauses: vec![GeneratedClause {
			predicate: "go".to_owned(),
			arguments: vec![crate::schema::GeneratedArgument {
				role: "Agent".to_owned(),
				referent: Some(0),
				clause: Some(0),
			}],
			..Default::default()
		}],
		roots: vec![0],
	};
	let utterance = generated.into_overlay_utterance()?;
	let clause = utterance.clauses.values().next().context("clause")?;
	assert!(matches!(
		clause.arguments[0].value,
		maybraid_language_core::SemanticValue::Referent(_)
	));
	Ok(())
}
