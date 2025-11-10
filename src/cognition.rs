use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};
use std::fmt;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum CogState {
    Think,
    Decide,
    Act,
    Observe,
    Reflect,
}

impl fmt::Display for CogState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CogState::Think => write!(f, "Think"),
            CogState::Decide => write!(f, "Decide"),
            CogState::Act => write!(f, "Act"),
            CogState::Observe => write!(f, "Observe"),
            CogState::Reflect => write!(f, "Reflect"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CogStep {
    pub state: CogState,
    pub content: String,
    pub timestamp: i64,
    pub related_task: Option<u64>,
}

impl CogStep {
    pub fn new(state: CogState, content: String, related_task: Option<u64>) -> Self {
        Self {
            state,
            content,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            related_task,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Task {
    pub id: u64,
    pub goal: String,
    pub status: String,
    pub priority: u8,
}

impl Task {
    pub fn new(id: u64, goal: String, priority: u8) -> Self {
        Self {
            id,
            goal,
            status: "open".to_string(),
            priority,
        }
    }
}