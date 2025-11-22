//! TOON Decoder
//!
//! Parses LLM JSON output into TOON instructions

use super::ToonInstr;
use serde::{Deserialize, Serialize};

pub struct ToonDecoder {
    _private: (),
}

#[derive(Debug, Deserialize, Serialize)]
struct LlmOutput {
    ops: Vec<OpJson>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum OpJson {
    LoadMemory { id: String },
    Retrieve { query: String, k: usize },
    FoldContext { context_ids: Vec<String> },
    EmitPointer { id: String },
    Noop,
}

impl ToonDecoder {
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Decode LLM JSON output into TOON instructions
    ///
    /// Expected format:
    /// ```json
    /// {
    ///   "ops": [
    ///     {"type": "load_memory", "id": "M123"},
    ///     {"type": "retrieve", "query": "search", "k": 5},
    ///     {"type": "fold_context", "context_ids": ["id1", "id2"]},
    ///     {"type": "emit_pointer", "id": "P456"},
    ///     {"type": "noop"}
    ///   ]
    /// }
    /// ```
    pub fn decode_ops(&self, json: &str) -> Result<Vec<ToonInstr>, String> {
        // Parse JSON
        let output: LlmOutput =
            serde_json::from_str(json).map_err(|e| format!("Failed to parse JSON: {}", e))?;

        // Convert OpJson to ToonInstr
        let mut instructions = Vec::new();
        for op in output.ops {
            let instr = match op {
                OpJson::LoadMemory { id } => ToonInstr::LoadMemory { id },
                OpJson::Retrieve { query, k } => ToonInstr::Retrieve { query, k },
                OpJson::FoldContext { context_ids } => ToonInstr::FoldContext { context_ids },
                OpJson::EmitPointer { id } => ToonInstr::EmitPointer { id },
                OpJson::Noop => ToonInstr::NoOp,
            };
            instructions.push(instr);
        }

        Ok(instructions)
    }
}

impl Default for ToonDecoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_all_types() {
        let decoder = ToonDecoder::new();

        let json = r#"{
            "ops": [
                {"type": "load_memory", "id": "M123"},
                {"type": "retrieve", "query": "search", "k": 3},
                {"type": "fold_context", "context_ids": ["id1", "id2"]},
                {"type": "emit_pointer", "id": "P456"},
                {"type": "noop"}
            ]
        }"#;

        let result = decoder.decode_ops(json);
        assert!(result.is_ok());

        let ops = result.unwrap();
        assert_eq!(ops.len(), 5);

        assert!(matches!(ops[0], ToonInstr::LoadMemory { .. }));
        assert!(matches!(ops[1], ToonInstr::Retrieve { .. }));
        assert!(matches!(ops[2], ToonInstr::FoldContext { .. }));
        assert!(matches!(ops[3], ToonInstr::EmitPointer { .. }));
        assert!(matches!(ops[4], ToonInstr::NoOp));
    }

    #[test]
    fn test_decode_invalid_json() {
        let decoder = ToonDecoder::new();

        let invalid = "{ not valid json";
        let result = decoder.decode_ops(invalid);

        assert!(result.is_err());
    }

    #[test]
    fn test_decode_deterministic() {
        let decoder = ToonDecoder::new();

        let json = r#"{
            "ops": [
                {"type": "retrieve", "query": "test", "k": 3},
                {"type": "emit_pointer", "id": "P1"}
            ]
        }"#;

        let ops1 = decoder.decode_ops(json).unwrap();
        let ops2 = decoder.decode_ops(json).unwrap();

        assert_eq!(ops1.len(), ops2.len());

        // Check first instruction
        match (&ops1[0], &ops2[0]) {
            (
                ToonInstr::Retrieve { query: q1, k: k1 },
                ToonInstr::Retrieve { query: q2, k: k2 },
            ) => {
                assert_eq!(q1, q2);
                assert_eq!(k1, k2);
            }
            _ => panic!("Instructions should match"),
        }
    }

    #[test]
    fn test_decode_requires_ops_field() {
        let decoder = ToonDecoder::new();

        let json = r#"{"operations": []}"#;
        let result = decoder.decode_ops(json);

        assert!(result.is_err());
    }

    #[test]
    fn test_decode_empty_ops() {
        let decoder = ToonDecoder::new();

        let json = r#"{"ops": []}"#;
        let result = decoder.decode_ops(json);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }
}
