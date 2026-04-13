use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub(super) struct HealthResponse {
    pub(super) status: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct SessionResponse {
    pub(super) id: String,
    pub(super) parent_session_id: Option<String>,
    pub(super) fork_point: Option<i64>,
    pub(super) created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ForkResponse {
    pub(super) new_session_id: String,
    pub(super) parent_session_id: String,
    pub(super) fork_point: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(super) struct SessionMessage {
    pub(super) id: String,
    pub(super) actor_type: String,
    pub(super) actor_id: String,
    pub(super) session_seq: i64,
    pub(super) content_text: String,
    pub(super) state: String,
    pub(super) created_at: String,
}

#[derive(Debug, Serialize)]
pub(super) struct HealthOutput<'a> {
    pub(super) status: &'a str,
    pub(super) base_url: Option<&'a str>,
}

#[derive(Debug, Serialize)]
pub(super) struct ChatOutput<'a> {
    pub(super) session_id: &'a str,
    pub(super) output_text: &'a str,
}

#[derive(Debug, Serialize)]
pub(super) struct SessionMemoryOutput {
    pub(super) session_id: String,
    pub(super) memory: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) memory_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) updated_at: Option<String>,
    pub(super) exists: bool,
}

#[derive(Debug, Deserialize)]
pub(super) struct SessionMessagesResponse {
    pub(super) messages: Vec<SessionMessage>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct SessionCompactResponse {
    pub(super) id: String,
    pub(super) turn_id: String,
    pub(super) summary: String,
    pub(super) start_session_seq: i64,
    pub(super) end_session_seq: i64,
    pub(super) created_at: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct SessionCompactsResponse {
    pub(super) compacts: Vec<SessionCompactResponse>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(super) struct SessionEffect {
    pub(super) id: String,
    pub(super) session_id: String,
    pub(super) effect_type: String,
    pub(super) idempotency_key: String,
    pub(super) status: String,
    pub(super) source_hook_id: String,
    pub(super) source_turn_id: String,
    pub(super) result_ref: Option<String>,
    pub(super) error_text: Option<String>,
    pub(super) created_at: String,
    pub(super) updated_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct SoulMemoryResponse {
    pub(super) id: String,
    pub(super) memory: String,
    pub(super) updated_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct SoulResponse {
    pub(super) id: String,
    pub(super) memory: String,
    pub(super) created_at: String,
    pub(super) updated_at: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct SessionEffectsResponse {
    pub(super) effects: Vec<SessionEffect>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct SessionMemoryResponse {
    pub(super) id: String,
    pub(super) memory: String,
    pub(super) updated_at: String,
}

impl SessionMemoryResponse {
    pub(super) fn into_output(self, session_id: &str) -> SessionMemoryOutput {
        SessionMemoryOutput {
            session_id: session_id.to_owned(),
            memory: self.memory,
            memory_id: Some(self.id),
            updated_at: Some(self.updated_at),
            exists: true,
        }
    }
}

impl SessionMemoryOutput {
    pub(super) fn empty(session_id: &str) -> Self {
        Self {
            session_id: session_id.to_owned(),
            memory: String::new(),
            memory_id: None,
            updated_at: None,
            exists: false,
        }
    }
}
