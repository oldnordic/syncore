//! Reasoning Node Context - ST-3
//!
//! Provides context and evaluation for individual thought nodes.
//! Handles node loading, prompt preparation, and basic evaluation.

use crate::databases::cognition_graph::{
    get_nodes_for_session, ThoughtNodeProperties, ThoughtNodeResult,
};
use crate::graph::Neo4jClient;
use crate::reasoning::{ReasoningError, ReasoningResult};

use std::sync::Arc;

/// Context for reasoning about a specific thought node
#[derive(Debug, Clone)]
pub struct ReasoningNodeContext {
    /// The thought node this context is for
    pub node: ThoughtNodeProperties,

    /// Session ID this node belongs to
    pub session_id: String,

    /// Path from root to this node (for context)
    pub path: Vec<ThoughtNodeProperties>,

    /// Child nodes of this node
    pub children: Vec<ThoughtNodeProperties>,
}

impl ReasoningNodeContext {
    /// Load node context by ID
    ///
    /// Loads the node and its surrounding context (path and children).
    /// Returns error if node doesn't exist.
    pub async fn load(
        client: Arc<Neo4jClient>,
        session_id: &str,
        node_id: &str,
    ) -> ReasoningResult<Self> {
        // Get all nodes in session to build context
        let all_nodes =
            get_nodes_for_session(&client, session_id).await.map_err(ReasoningError::Database)?;

        // Find the target node
        let target_node = all_nodes
            .iter()
            .find(|node| node.id == node_id)
            .ok_or_else(|| ReasoningError::NodeNotFound(node_id.to_string()))?;

        let node_props = ThoughtNodeProperties {
            id: target_node.id.clone(),
            session_id: target_node.session_id.clone(),
            parent_id: target_node.parent_id.clone(),
            step_index: target_node.step_index,
            content: target_node.content.clone(),
            score: target_node.score,
        };

        // Build path from root to this node
        let path = Self::build_path_to_node(&all_nodes, &node_props);

        // Find children of this node
        let children = all_nodes
            .iter()
            .filter(|node| node.parent_id.as_ref() == Some(&node_props.id))
            .map(|result| ThoughtNodeProperties {
                id: result.id.clone(),
                session_id: result.session_id.clone(),
                parent_id: result.parent_id.clone(),
                step_index: result.step_index,
                content: result.content.clone(),
                score: result.score,
            })
            .collect();

        Ok(Self {
            node: node_props,
            session_id: session_id.to_string(),
            path,
            children,
        })
    }

    /// Build the path from root to the given node
    fn build_path_to_node(
        all_nodes: &[ThoughtNodeResult],
        target: &ThoughtNodeProperties,
    ) -> Vec<ThoughtNodeProperties> {
        let mut path = Vec::new();
        let mut current_id = Some(target.id.clone());

        // Walk up the tree from target to root
        while let Some(node_id) = current_id {
            if let Some(node) = all_nodes.iter().find(|n| n.id == node_id) {
                let node_props = ThoughtNodeProperties {
                    id: node.id.clone(),
                    session_id: node.session_id.clone(),
                    parent_id: node.parent_id.clone(),
                    step_index: node.step_index,
                    content: node.content.clone(),
                    score: node.score,
                };

                path.insert(0, node_props); // Insert at beginning to maintain root-to-leaf order
                current_id = node.parent_id.clone();
            } else {
                break; // Parent not found (shouldn't happen in valid trees)
            }
        }

        path
    }

    /// Prepare comprehensive LLM prompt context for node expansion
    ///
    /// Creates a detailed prompt context that includes session history,
    /// reasoning path, current node content, and structural context.
    /// This provides the LLM with complete context for generating meaningful expansions.
    pub fn prepare_prompt_context(&self) -> String {
        let mut context = String::new();

        // Add session and depth context
        context.push_str(&format!("Reasoning Session: {}\n", self.session_id));
        context.push_str(&format!("Current Depth: {}\n", self.path.len()));

        // Add reasoning path with context
        if !self.path.is_empty() {
            context.push_str("Previous Reasoning Steps:\n");
            for (i, node) in self.path.iter().enumerate() {
                context.push_str(&format!(
                    "  Step {}: [{}] Score: {:.2} - {}\n",
                    i + 1,
                    "thought", // node_type field doesn't exist
                    node.score.unwrap_or(0.5),
                    node.content
                ));
            }
        } else {
            context.push_str("Starting new reasoning chain.\n");
        }

        // Add current node with rich context
        context.push_str("\n=== CURRENT NODE FOR EXPANSION ===\n");
        context.push_str(&format!("Content: {}\n", self.node.content));
        context.push_str(&format!("Current Score: {:.2}\n", self.node.score.unwrap_or(0.5)));

        // Add step index as additional context
        context.push_str(&format!("Step Index: {}\n", self.node.step_index));

        // Add existing branches if any
        if !self.children.is_empty() {
            context.push_str("\nExisting Branches:\n");
            for (i, child) in self.children.iter().enumerate() {
                context.push_str(&format!(
                    "  Branch {}: [{}] Score: {:.2} - {}\n",
                    i + 1,
                    "thought", // node_type field doesn't exist
                    child.score.unwrap_or(0.5),
                    child.content
                ));
            }
        }

        // Add guidance for LLM
        context.push_str("\n=== EXPANSION GUIDANCE ===\n");
        context.push_str("Generate 3-5 diverse reasoning branches that explore different:\n");
        context.push_str("- Perspectives or approaches to the current problem\n");
        context.push_str("- Potential next steps or questions\n");
        context.push_str("- Alternative interpretations or hypotheses\n");
        context.push_str("Each branch should be concrete and actionable.\n");

        context
    }

    /// Evaluate node quality using local heuristics and optional LLM scoring
    ///
    /// Provides a quality score based on content structure, context richness,
    /// and reasoning depth. When an LLM is available, uses it for semantic quality assessment.
    pub fn evaluate_quality(&self) -> f64 {
        // Structural quality metrics (always available)
        let content_len = self.node.content.len();

        // Base score from content length (prefer moderate length)
        let length_score = if content_len < 10 {
            0.1 // Too short
        } else if content_len < 50 {
            0.5 // Good length
        } else if content_len < 200 {
            0.8 // Good length
        } else {
            0.6 // Too long
        };

        // Content quality indicators
        let question_words = ["what", "how", "why", "when", "where", "which", "who"];
        let has_question = question_words.iter().any(|&word| {
            self.node.content.to_lowercase().contains(word) && self.node.content.contains('?')
        });
        let question_bonus = if has_question {
            0.1
        } else {
            0.0
        };

        // Action words indicate concrete reasoning
        let action_words =
            ["analyze", "evaluate", "consider", "implement", "test", "verify", "design"];
        let has_action =
            action_words.iter().any(|&word| self.node.content.to_lowercase().contains(word));
        let action_bonus = if has_action {
            0.1
        } else {
            0.0
        };

        // Context richness bonuses
        let children_bonus = if self.children.is_empty() {
            0.0
        } else {
            0.05 * (self.children.len() as f64).min(0.2) // Diminishing returns
        };

        // Reasoning depth bonus (deeper chains show more sophisticated reasoning)
        let depth_bonus = (self.path.len() as f64).min(5.0) * 0.03;

        // Step-based quality scores (using step index as proxy for node type)
        let step_score = match self.node.step_index {
            0..=2 => 0.15, // Early steps (exploration)
            3..=5 => 0.1,  // Middle steps (analysis)
            _ => 0.05,     // Later steps (conclusions)
        };

        // Combine all scores
        let total_score = length_score
            + question_bonus
            + action_bonus
            + children_bonus
            + depth_bonus
            + step_score;

        total_score.min(1.0)
    }

    /// Evaluate node quality using LLM for semantic assessment
    ///
    /// When an LLM is available, provides more sophisticated quality evaluation
    /// based on semantic coherence, relevance, and reasoning value.
    pub async fn evaluate_quality_with_llm(&self, llm: &dyn crate::llm::LanguageModel) -> f64 {
        use crate::llm::Prompt;

        let evaluation_prompt = Prompt {
            user: format!(
                "Evaluate the quality of this reasoning node on a scale of 0.0 to 1.0.\n\n\
                Content: {}\n\
                Score: {:.2}\n\
                Step Index: {}\n\
                Path Depth: {}\n\n\
                Consider:\n\
                1. Is the content clear and specific?\n\
                2. Does it advance the reasoning process?\n\
                3. Is it well-formulated and coherent?\n\
                4. Does it have good reasoning value?\n\n\
                Respond with only a number between 0.0 and 1.0.",
                self.node.content,
                self.node.score.unwrap_or(0.5),
                self.node.step_index,
                self.path.len()
            ),
            system: "You are a reasoning quality evaluator. Respond with only a numeric score."
                .to_string(),
            max_tokens: Some(10),
            temperature: Some(0.0),
        };

        match llm.complete(&evaluation_prompt) {
            Ok(completion) => {
                // Extract numeric score from response
                let response_text = completion.text.trim();
                // Try to parse as float
                response_text
                    .parse::<f64>()
                    .unwrap_or_else(|_| {
                        // Fallback: extract first number found
                        response_text
                            .chars()
                            .filter(|c| c.is_numeric() || *c == '.')
                            .collect::<String>()
                            .parse::<f64>()
                            .unwrap_or(0.5) // Default fallback
                    })
                    .clamp(0.0, 1.0)
            }
            Err(_) => {
                // Fallback to heuristic evaluation
                self.evaluate_quality()
            }
        }
    }

    /// Check if this node is a leaf (has no children)
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    /// Check if this node is the root (has no parent)
    pub fn is_root(&self) -> bool {
        self.node.parent_id.is_none()
    }

    /// Get depth of this node in the reasoning tree
    pub fn depth(&self) -> usize {
        self.path.len()
    }

    /// Get number of children
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// Create a new node context from existing properties
    pub fn from_properties(
        node: ThoughtNodeProperties,
        session_id: String,
        path: Vec<ThoughtNodeProperties>,
        children: Vec<ThoughtNodeProperties>,
    ) -> Self {
        Self {
            node,
            session_id,
            path,
            children,
        }
    }
}

/// Node evaluation utilities
pub struct NodeEvaluator;

impl NodeEvaluator {
    /// Evaluate multiple nodes and return rankings
    pub fn evaluate_nodes(nodes: &[ReasoningNodeContext]) -> Vec<(usize, f64)> {
        let mut evaluations: Vec<(usize, f64)> =
            nodes.iter().enumerate().map(|(i, context)| (i, context.evaluate_quality())).collect();

        // Sort by score descending
        evaluations.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        evaluations
    }

    /// Select best node from candidates
    pub fn select_best_node(nodes: &[ReasoningNodeContext]) -> Option<&ReasoningNodeContext> {
        if nodes.is_empty() {
            return None;
        }

        let evaluations = Self::evaluate_nodes(nodes);
        let best_index = evaluations[0].0;

        Some(&nodes[best_index])
    }

    /// Filter nodes by minimum quality threshold
    pub fn filter_by_quality(
        nodes: &[ReasoningNodeContext],
        min_score: f64,
    ) -> Vec<&ReasoningNodeContext> {
        nodes.iter().filter(|context| context.evaluate_quality() >= min_score).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::databases::cognition_graph::ThoughtNodeProperties;

    fn create_test_node(id: &str, content: &str, parent_id: Option<&str>) -> ThoughtNodeProperties {
        ThoughtNodeProperties {
            id: id.to_string(),
            session_id: "test_session".to_string(),
            parent_id: parent_id.map(|s| s.to_string()),
            step_index: 0,
            content: content.to_string(),
            score: None,
        }
    }

    #[test]
    fn test_reasoning_node_context_creation() {
        let node = create_test_node("node1", "Test content", None);
        let context = ReasoningNodeContext::from_properties(
            node,
            "test_session".to_string(),
            Vec::new(),
            Vec::new(),
        );

        assert_eq!(context.node.id, "node1");
        assert_eq!(context.session_id, "test_session");
        assert!(context.path.is_empty());
        assert!(context.children.is_empty());
        assert!(context.is_root());
        assert!(context.is_leaf());
        assert_eq!(context.depth(), 0);
        assert_eq!(context.child_count(), 0);
    }

    #[test]
    fn test_prompt_context_preparation() {
        let node = create_test_node("node1", "Current thinking", None);
        let context = ReasoningNodeContext::from_properties(
            node,
            "test_session".to_string(),
            Vec::new(),
            Vec::new(),
        );

        let prompt = context.prepare_prompt_context();
        assert!(prompt.contains("Session: test_session"));
        assert!(prompt.contains("Current Node: Current thinking"));
    }

    #[test]
    fn test_node_evaluation() {
        // Test short content (low score)
        let short_node = create_test_node("short", "Hi", None);
        let short_context = ReasoningNodeContext::from_properties(
            short_node,
            "test_session".to_string(),
            Vec::new(),
            Vec::new(),
        );
        assert!(short_context.evaluate_quality() < 0.5);

        // Test good length content (higher score)
        let good_node =
            create_test_node("good", "This is a good length content for reasoning", None);
        let good_context = ReasoningNodeContext::from_properties(
            good_node,
            "test_session".to_string(),
            Vec::new(),
            Vec::new(),
        );
        assert!(good_context.evaluate_quality() >= 0.5);
    }

    #[test]
    fn test_node_evaluator() {
        let contexts = vec![
            ReasoningNodeContext::from_properties(
                create_test_node("node1", "Short", None),
                "test_session".to_string(),
                Vec::new(),
                Vec::new(),
            ),
            ReasoningNodeContext::from_properties(
                create_test_node(
                    "node2",
                    "This is a much better content for reasoning purposes",
                    None,
                ),
                "test_session".to_string(),
                Vec::new(),
                Vec::new(),
            ),
        ];

        let evaluations = NodeEvaluator::evaluate_nodes(&contexts);
        assert_eq!(evaluations.len(), 2);

        // Higher score should be first
        assert!(evaluations[0].1 > evaluations[1].1);

        let best = NodeEvaluator::select_best_node(&contexts);
        assert!(best.is_some());
        assert_eq!(best.unwrap().node.id, "node2");
    }

    #[test]
    fn test_path_building() {
        // This tests the path building logic with mock data
        let root = create_test_node("root", "Root", None);
        let child = create_test_node("child", "Child", Some("root"));
        let grandchild = create_test_node("grandchild", "Grandchild", Some("child"));

        let all_nodes = vec![
            ThoughtNodeResult {
                id: root.id.clone(),
                session_id: root.session_id.clone(),
                parent_id: root.parent_id.clone(),
                step_index: root.step_index,
                content: root.content.clone(),
                score: root.score,
            },
            ThoughtNodeResult {
                id: child.id.clone(),
                session_id: child.session_id.clone(),
                parent_id: child.parent_id.clone(),
                step_index: child.step_index,
                content: child.content.clone(),
                score: child.score,
            },
            ThoughtNodeResult {
                id: grandchild.id.clone(),
                session_id: grandchild.session_id.clone(),
                parent_id: grandchild.parent_id.clone(),
                step_index: grandchild.step_index,
                content: grandchild.content.clone(),
                score: grandchild.score,
            },
        ];

        let path = ReasoningNodeContext::build_path_to_node(&all_nodes, &grandchild);
        assert_eq!(path.len(), 3);
        assert_eq!(path[0].id, "root");
        assert_eq!(path[1].id, "child");
        assert_eq!(path[2].id, "grandchild");
    }
}
