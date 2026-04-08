use crate::types::{QueryCase, Session, SessionTurn, TaskType};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

pub fn generate_sessions(seed: u64, count: usize) -> Vec<Session> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut sessions = Vec::with_capacity(count);

    for i in 0..count {
        let session_id = format!("session-{i:04}");
        let color_v1 = format!("color-{}", rng.gen_range(1..=16));
        let color_v2 = format!("shade-{}", rng.gen_range(1..=16));
        let city = format!("city-{}", rng.gen_range(1..=32));

        let turns = vec![
            SessionTurn {
                ts: "2026-01-01T10:00:00Z".into(),
                speaker: "user".into(),
                text: format!("My favorite color is {color_v1}."),
                memory_id: Some(format!("{session_id}:favorite_color:v1")),
            },
            SessionTurn {
                ts: "2026-01-03T10:00:00Z".into(),
                speaker: "user".into(),
                text: format!("I moved to {city}."),
                memory_id: Some(format!("{session_id}:city:v1")),
            },
            SessionTurn {
                ts: "2026-01-04T10:00:00Z".into(),
                speaker: "user".into(),
                text: format!("Correction: my favorite color is now {color_v2}."),
                memory_id: Some(format!("{session_id}:favorite_color:v2")),
            },
        ];

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
            query: "Which favorite color should replace the older value?".into(),
            gold_evidence_ids: vec![format!("{}:favorite_color:v2", s.session_id)],
            expected_answer: None,
            should_abstain: false,
        });

        cases.push(QueryCase {
            query_id: format!("{}:temporal", s.session_id),
            session_id: s.session_id.clone(),
            task_type: TaskType::Temporal,
            query: "Where did I live after the move?".into(),
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

    cases
}
