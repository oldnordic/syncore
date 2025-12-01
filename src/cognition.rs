//! Cognition Module
//!
//! Provides cognitive processing capabilities including:
//! - Intent classification
//! - Routing logic
//! - Query orchestration

use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

// Phase R3.1: Cognitive routing and intent classification
pub mod intent_classifier;
pub mod orchestrator;
pub mod router_logic;

// Phase R3.2: Multi-tool context fusion
pub mod context_bundle;
pub mod context_composer;

// Phase R3.3: Reasoning continuity engine
pub mod continuity_engine;
pub mod reasoning_ledger;

// Phase R4.1: Reasoning pattern extraction
pub mod pattern_engine;

// Phase R4.2: Self-consistency checker
pub mod self_consistency;
pub mod self_consistency_types;

// Phase R5.0: Planning engine + plan executor
pub mod plan_engine;
pub mod plan_executor;

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
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
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
