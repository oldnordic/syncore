use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum SynCoreTool {
    MemoryStore,
    MemoryQuery,
    VectorInsert,
    VectorSearch,
    LogsTail,
    TaskCreate,
    GraphLink,
    GraphQuery,

    ParserAnalyze,
    ParserSearch,
    CodeExplain,
    CodeIndexDirectory,
}

impl SynCoreTool {
    pub fn to_string(&self) -> String {
        match self {
            SynCoreTool::MemoryStore => "memory.store".to_string(),
            SynCoreTool::MemoryQuery => "memory.query".to_string(),
            SynCoreTool::VectorInsert => "vector.insert".to_string(),
            SynCoreTool::VectorSearch => "vector.search".to_string(),
            SynCoreTool::LogsTail => "logs.tail".to_string(),
            SynCoreTool::TaskCreate => "task.create".to_string(),
            SynCoreTool::GraphLink => "graph.link".to_string(),
            SynCoreTool::GraphQuery => "graph.query".to_string(),

            SynCoreTool::ParserAnalyze => "parser.analyze".to_string(),
            SynCoreTool::ParserSearch => "parser.search".to_string(),
            SynCoreTool::CodeExplain => "code.explain".to_string(),
            SynCoreTool::CodeIndexDirectory => "code.index_directory".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SynCoreMsg {
    pub tool: SynCoreTool,
    pub args: Vec<u8>, // MessagePack-encoded inner payload
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syn_core_tool_serialization() {
        // Test that SynCoreTool can be serialized and deserialized
        let tool = SynCoreTool::MemoryStore;
        let serialized = serde_json::to_string(&tool).unwrap();
        let deserialized: SynCoreTool = serde_json::from_str(&serialized).unwrap();

        assert_eq!(tool, deserialized, "SynCoreTool should serialize/deserialize correctly");
    }

    #[test]
    fn test_syn_core_msg_serialization() {
        // Test that SynCoreMsg can be serialized and deserialized
        let msg = SynCoreMsg {
            tool: SynCoreTool::VectorSearch,
            args: vec![1, 2, 3, 4, 5],
        };

        let serialized = serde_json::to_string(&msg).unwrap();
        let deserialized: SynCoreMsg = serde_json::from_str(&serialized).unwrap();

        assert_eq!(msg.tool, deserialized.tool, "Tool should serialize/deserialize correctly");
        assert_eq!(msg.args, deserialized.args, "Args should serialize/deserialize correctly");
    }

    #[test]
    fn test_all_syn_core_tools() {
        // Test that all SynCoreTool variants can be serialized
        let tools = vec![
            SynCoreTool::MemoryStore,
            SynCoreTool::MemoryQuery,
            SynCoreTool::VectorInsert,
            SynCoreTool::VectorSearch,
            SynCoreTool::LogsTail,
            SynCoreTool::TaskCreate,
            SynCoreTool::GraphLink,
            SynCoreTool::GraphQuery,
            SynCoreTool::ParserAnalyze,
            SynCoreTool::ParserSearch,
            SynCoreTool::CodeExplain,
            SynCoreTool::CodeIndexDirectory,
        ];

        for tool in tools {
            let serialized = serde_json::to_string(&tool);
            assert!(serialized.is_ok(), "Tool {:?} should serialize", tool);

            let serialized_str = serialized.unwrap();
            let deserialized: Result<SynCoreTool, _> = serde_json::from_str(&serialized_str);
            assert!(deserialized.is_ok(), "Tool {:?} should deserialize", tool);

            assert_eq!(tool, deserialized.unwrap(), "Tool {:?} should round-trip correctly", tool);
        }
    }

    #[test]
    fn test_syn_core_msg_debug_format() {
        // Test that SynCoreMsg has a reasonable debug format
        let msg = SynCoreMsg {
            tool: SynCoreTool::TaskCreate,
            args: vec![10, 20, 30],
        };

        let debug_str = format!("{:?}", msg);
        assert!(debug_str.contains("TaskCreate"), "Debug string should contain tool name");
        assert!(debug_str.contains("[10, 20, 30]"), "Debug string should contain args");
    }

    #[test]
    fn test_syn_core_tool_debug_format() {
        // Test that SynCoreTool has a reasonable debug format
        let tools =
            vec![SynCoreTool::MemoryStore, SynCoreTool::VectorSearch, SynCoreTool::TaskCreate];

        for tool in tools {
            let debug_str = format!("{:?}", tool);
            assert!(!debug_str.is_empty(), "Debug string for {:?} should not be empty", tool);
        }
    }
}
