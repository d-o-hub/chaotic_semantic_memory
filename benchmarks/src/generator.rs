use crate::types::{QueryCase, Session, SessionTurn, TaskType};
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Real colors for semantically meaningful test data
const COLORS: &[&str] = &[
    "blue", "green", "red", "purple", "orange", "yellow", "pink", "teal", "navy", "maroon",
    "olive", "aqua", "lime", "coral", "crimson", "indigo",
];

/// Color modifiers for variations
const COLOR_MODIFIERS: &[&str] = &[
    "light", "dark", "bright", "pale", "vibrant", "soft", "deep", "muted",
];

/// Real pets for semantically meaningful test data
const PETS: &[&str] = &[
    "Golden Retriever",
    "Siamese Cat",
    "Hamster",
    "Parrot",
    "Iguana",
    "Rabbit",
    "Goldfish",
    "Beagle",
    "Persian Cat",
    "Cockatiel",
    "Turtle",
    "Chinchilla",
    "Ferret",
    "Hedgehog",
];

/// Real cities for semantically meaningful test data
const CITIES: &[&str] = &[
    "New York",
    "Los Angeles",
    "Chicago",
    "Houston",
    "Phoenix",
    "Philadelphia",
    "San Antonio",
    "San Diego",
    "Dallas",
    "San Jose",
    "Austin",
    "Jacksonville",
    "Fort Worth",
    "Columbus",
    "Charlotte",
    "Seattle",
    "Denver",
    "Boston",
    "Nashville",
    "Baltimore",
    "Oklahoma City",
    "Louisville",
    "Portland",
    "Vegas",
    "Milwaukee",
    "Albuquerque",
    "Tucson",
    "Fresno",
    "Sacramento",
    "Kansas City",
    "Atlanta",
    "Miami",
];

/// Generate sessions with variable turn count between min and max (inclusive)
pub fn generate_sessions_with_range(
    seed: u64,
    count: usize,
    min_turns: usize,
    max_turns: usize,
) -> Vec<Session> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut sessions = Vec::with_capacity(count);

    for i in 0..count {
        let session_id = format!("session-{i:04}");
        let turn_count = if min_turns == max_turns {
            min_turns
        } else {
            rng.random_range(min_turns..=max_turns)
        };

        let color_v1 = COLORS[rng.random_range(0..COLORS.len())];
        let color_mod = COLOR_MODIFIERS[rng.random_range(0..COLOR_MODIFIERS.len())];
        let color_v2 = format!(
            "{} {}",
            color_mod,
            COLORS[rng.random_range(0..COLORS.len())]
        );
        let city = CITIES[rng.random_range(0..CITIES.len())];
        let pet = PETS[rng.random_range(0..PETS.len())];

        // Build turns based on turn_count (minimum 1 turn)
        let mut turns = Vec::with_capacity(turn_count);

        // First turn always mentions color
        turns.push(SessionTurn {
            ts: "2026-01-01T10:00:00Z".into(),
            speaker: "user".into(),
            text: format!("My favorite color is {color_v1}."),
            memory_id: Some(format!("{session_id}:favorite_color:v1")),
        });

        // Second turn mentions city (if we have at least 2 turns)
        if turn_count >= 2 {
            turns.push(SessionTurn {
                ts: "2026-01-03T10:00:00Z".into(),
                speaker: "user".into(),
                text: format!("I moved to {city}."),
                memory_id: Some(format!("{session_id}:city:v1")),
            });
        }

        // Mention a pet (if we have at least 4 turns)
        if turn_count >= 4 {
            turns.push(SessionTurn {
                ts: "2026-01-05T10:00:00Z".into(),
                speaker: "user".into(),
                text: format!("I have a {pet} as a pet."),
                memory_id: Some(format!("{session_id}:pet:v1")),
            });
        }

        // Third turn updates color (if we have at least 3 turns)
        if turn_count >= 3 {
            turns.push(SessionTurn {
                ts: "2026-01-04T10:00:00Z".into(),
                speaker: "user".into(),
                text: format!(
                    "Actually, I changed my mind. My current favorite color is {color_v2} now."
                ),
                memory_id: Some(format!("{session_id}:favorite_color:v2")),
            });
        }

        // Additional turns with filler content (for variable-length stress testing)
        let start_filler = if turn_count >= 4 { 4 } else { 3 };
        for j in start_filler..turn_count {
            let filler_idx = j - start_filler;
            turns.push(SessionTurn {
                ts: format!("2026-01-{:02}T10:00:00Z", 5 + filler_idx),
                speaker: "user".into(),
                text: format!(
                    "I also wanted to mention something about topic {}.",
                    filler_idx + 1
                ),
                memory_id: Some(format!("{session_id}:topic:{}:v1", filler_idx + 1)),
            });
        }

        sessions.push(Session { session_id, turns });
    }

    sessions
}

pub fn generate_queries(sessions: &[Session]) -> Vec<QueryCase> {
    let mut cases = Vec::new();

    for s in sessions {
        cases.push(QueryCase {
            query_id: format!("{}:recall", s.session_id),
            session_id: s.session_id.clone(),
            task_type: TaskType::Recall,
            query: "What is my current favorite color?".into(),
            gold_evidence_ids: vec![format!("{}:favorite_color:v2", s.session_id)],
            expected_answer: None,
            should_abstain: false,
        });

        cases.push(QueryCase {
            query_id: format!("{}:update", s.session_id),
            session_id: s.session_id.clone(),
            task_type: TaskType::Update,
            query: "What is my favorite color now?".into(), // Uses "now" which appears in v2
            gold_evidence_ids: vec![format!("{}:favorite_color:v2", s.session_id)],
            expected_answer: None,
            should_abstain: false,
        });

        cases.push(QueryCase {
            query_id: format!("{}:temporal", s.session_id),
            session_id: s.session_id.clone(),
            task_type: TaskType::Temporal,
            query: "What city did I move to?".into(), // Uses "city" and "move" keywords
            gold_evidence_ids: vec![format!("{}:city:v1", s.session_id)],
            expected_answer: None,
            should_abstain: false,
        });

        cases.push(QueryCase {
            query_id: format!("{}:abstain", s.session_id),
            session_id: s.session_id.clone(),
            task_type: TaskType::Abstain,
            query: "What is my passport number?".into(),
            gold_evidence_ids: vec![],
            expected_answer: None,
            should_abstain: true,
        });
    }

    // Add cross-session query types (Association and MultiSession)
    if sessions.len() >= 2 {
        // Association: Link concepts across sessions by finding common themes
        for i in 0..sessions.len().min(10) {
            let s1 = &sessions[i];
            let s2 = &sessions[(i + 1) % sessions.len()];

            // Generate association query based on shared color or city if possible,
            // but for simplicity we'll just use colors from these two sessions.
            // Note: color v1 is pruned if v2 exists, so we use v2.
            cases.push(QueryCase {
                query_id: format!("association-{:03}", i),
                session_id: "cross-session".into(),
                task_type: TaskType::Association,
                query: "What are the favorite colors I've mentioned in my different chats?".into(),
                gold_evidence_ids: vec![
                    format!("{}:favorite_color:v2", s1.session_id),
                    format!("{}:favorite_color:v2", s2.session_id),
                ],
                expected_answer: None,
                should_abstain: false,
            });

            // Add a query that targets explicit associations between city and color in a session
            cases.push(QueryCase {
                query_id: format!("association-internal-{:03}", i),
                session_id: s1.session_id.clone(),
                task_type: TaskType::Association,
                query: "Show me items related to my location city and favorite interests color in this session.".into(),
                gold_evidence_ids: vec![
                    format!("{}:favorite_color:v2", s1.session_id),
                    format!("{}:city:v1", s1.session_id),
                ],
                expected_answer: None,
                should_abstain: false,
            });
        }

        // MultiSession: Aggregate across sessions
        cases.push(QueryCase {
            query_id: "cross-session:multisession-city".into(),
            session_id: "cross-session".into(),
            task_type: TaskType::MultiSession,
            query: "Which cities have I lived in or moved to as a resident?".into(),
            gold_evidence_ids: sessions
                .iter()
                .filter_map(|s| {
                    s.turns
                        .iter()
                        .find(|t| t.memory_id.as_ref().is_some_and(|id| id.contains(":city:")))
                })
                .filter_map(|t| t.memory_id.clone())
                .collect(),
            expected_answer: None,
            should_abstain: false,
        });

        cases.push(QueryCase {
            query_id: "cross-session:multisession-pet".into(),
            session_id: "cross-session".into(),
            task_type: TaskType::MultiSession,
            query: "What pets have I mentioned having?".into(),
            gold_evidence_ids: sessions
                .iter()
                .filter_map(|s| {
                    s.turns
                        .iter()
                        .find(|t| t.memory_id.as_ref().is_some_and(|id| id.contains(":pet:")))
                })
                .filter_map(|t| t.memory_id.clone())
                .collect(),
            expected_answer: None,
            should_abstain: false,
        });

        // Isolation: Query a session for data that belongs to another session
        for i in 0..sessions.len().min(5) {
            let s_target = &sessions[i];
            let s_other = &sessions[(i + 1) % sessions.len()];

            // Find a unique pet from the OTHER session
            let other_pet_turn = s_other
                .turns
                .iter()
                .find(|t| t.memory_id.as_ref().is_some_and(|id| id.contains(":pet:")));

            if let Some(turn) = other_pet_turn {
                // Extract pet name from "I have a [pet] as a pet."
                let pet_name = turn.text.replace("I have a ", "").replace(" as a pet.", "");

                cases.push(QueryCase {
                    query_id: format!("isolation-{:03}", i),
                    session_id: s_target.session_id.clone(),
                    task_type: TaskType::Isolation,
                    query: format!("Do I have a {}?", pet_name),
                    gold_evidence_ids: vec![], // Should NOT find anything
                    expected_answer: None,
                    should_abstain: true,
                });
            }
        }
    }

    cases
}
