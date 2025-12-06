//! Simple standalone test for the coercion function

#[cfg(test)]
mod tests {
    use serde_json::json;
    use syncore::mcp_tools::memory_suite::intellitask_commands::coerce_intellitask_payload;

    #[test]
    fn test_array_coercion() {
        let input = json!([
            {"id": "1", "goal": "Task 1"},
            {"id": "2", "goal": "Task 2"}
        ]);

        let result = coerce_intellitask_payload(input);
        assert!(result.is_object());
        assert_eq!(result.get("prd_title").unwrap().as_str().unwrap(), "Unknown PRD");
    }

    #[test]
    fn test_object_coercion() {
        let input = json!({
            "parent_tasks": []
        });

        let result = coerce_intellitask_payload(input);
        assert!(result.is_object());
        assert_eq!(result.get("estimated_complexity").unwrap().as_str().unwrap(), "Moderate");
    }
}
