use syncore::cognition::{CogState, CogStep, Task};

#[test]
fn test_cog_step_creation_and_serialization() {
    let step =
        CogStep::new(CogState::Think, "I need to optimize the SIMD path".to_string(), Some(42));

    // Test serialization
    let serialized = rmp_serde::to_vec(&step).unwrap();
    let deserialized: CogStep = rmp_serde::from_slice(&serialized).unwrap();

    assert_eq!(deserialized.state, CogState::Think);
    assert_eq!(deserialized.content, "I need to optimize the SIMD path");
    assert_eq!(deserialized.related_task, Some(42));
    assert!(deserialized.timestamp > 0);
}

#[test]
fn test_task_creation_and_serialization() {
    let task = Task::new(1, "Optimize SIMD kernel".to_string(), 8);

    // Test serialization
    let serialized = rmp_serde::to_vec(&task).unwrap();
    let deserialized: Task = rmp_serde::from_slice(&serialized).unwrap();

    assert_eq!(deserialized.id, 1);
    assert_eq!(deserialized.goal, "Optimize SIMD kernel");
    assert_eq!(deserialized.status, "open");
    assert_eq!(deserialized.priority, 8);
}

#[test]
fn test_cognitive_state_sequence() {
    let mut steps = Vec::new();

    // Create a sequence of cognitive steps
    steps.push(CogStep::new(
        CogState::Think,
        "Analyze performance bottleneck".to_string(),
        Some(1),
    ));
    steps.push(CogStep::new(CogState::Decide, "Implement fused kernel".to_string(), Some(1)));
    steps.push(CogStep::new(CogState::Act, "Wrote fused SIMD implementation".to_string(), Some(1)));
    steps.push(CogStep::new(CogState::Observe, "Speedup measured: 1.9x".to_string(), Some(1)));
    steps.push(CogStep::new(
        CogState::Reflect,
        "Goal achieved, task complete".to_string(),
        Some(1),
    ));

    // Verify sequence
    assert_eq!(steps.len(), 5);
    assert_eq!(steps[0].state, CogState::Think);
    assert_eq!(steps[1].state, CogState::Decide);
    assert_eq!(steps[2].state, CogState::Act);
    assert_eq!(steps[3].state, CogState::Observe);
    assert_eq!(steps[4].state, CogState::Reflect);

    // All steps should be related to the same task
    for step in &steps {
        assert_eq!(step.related_task, Some(1));
    }

    // Timestamps should be sequential
    for i in 1..steps.len() {
        assert!(steps[i].timestamp >= steps[i - 1].timestamp);
    }
}
