//! TDD Tests for TOON Decoder

use syncore::memory_service::{ToonDecoder, ToonInstr};

#[test]
fn test_decoder_parses_valid_json() {
    // Test basic JSON parsing
    let decoder = ToonDecoder::new();

    let json = r#"{
        "ops": [
            {"type": "retrieve", "query": "test", "k": 5}
        ]
    }"#;

    let result = decoder.decode_ops(json);
    assert!(result.is_ok(), "Should parse valid JSON");

    let ops = result.unwrap();
    assert_eq!(ops.len(), 1);

    match &ops[0] {
        ToonInstr::Retrieve {
            query,
            k,
        } => {
            assert_eq!(query, "test");
            assert_eq!(*k, 5);
        }
        _ => panic!("Expected Retrieve instruction"),
    }
}

#[test]
fn test_decoder_rejects_invalid_json() {
    // Test that malformed JSON is rejected
    let decoder = ToonDecoder::new();

    let invalid_json = r#"{ "ops": [ { "type": "retrieve", missing_bracket"#;

    let result = decoder.decode_ops(invalid_json);
    assert!(result.is_err(), "Should reject malformed JSON");
}

#[test]
fn test_decoder_rejects_unknown_fields() {
    // Test that unknown operation types cause errors
    let decoder = ToonDecoder::new();

    let json_with_unknown = r#"{
        "ops": [
            {"type": "unknown_operation", "data": "value"}
        ]
    }"#;

    let result = decoder.decode_ops(json_with_unknown);
    assert!(result.is_err(), "Should reject unknown operation types");
}

#[test]
fn test_decoder_parses_all_instr_types() {
    // Test that all instruction types can be parsed
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
    assert!(result.is_ok(), "Should parse all instruction types");

    let ops = result.unwrap();
    assert_eq!(ops.len(), 5, "Should have 5 instructions");

    // Verify each type
    assert!(matches!(ops[0], ToonInstr::LoadMemory { .. }));
    assert!(matches!(ops[1], ToonInstr::Retrieve { .. }));
    assert!(matches!(ops[2], ToonInstr::FoldContext { .. }));
    assert!(matches!(ops[3], ToonInstr::EmitPointer { .. }));
    assert!(matches!(ops[4], ToonInstr::NoOp));
}

#[test]
fn test_decoder_stable_ordering() {
    // Test that instruction order is preserved
    let decoder = ToonDecoder::new();

    let json = r#"{
        "ops": [
            {"type": "emit_pointer", "id": "P1"},
            {"type": "emit_pointer", "id": "P2"},
            {"type": "emit_pointer", "id": "P3"}
        ]
    }"#;

    let ops = decoder.decode_ops(json).unwrap();

    // Order should be preserved
    if let ToonInstr::EmitPointer {
        id,
    } = &ops[0]
    {
        assert_eq!(id, "P1");
    }
    if let ToonInstr::EmitPointer {
        id,
    } = &ops[1]
    {
        assert_eq!(id, "P2");
    }
    if let ToonInstr::EmitPointer {
        id,
    } = &ops[2]
    {
        assert_eq!(id, "P3");
    }
}

#[test]
fn test_decoder_requires_ops_field() {
    // Test that missing "ops" field causes error
    let decoder = ToonDecoder::new();

    let json = r#"{"operations": []}"#;

    let result = decoder.decode_ops(json);
    assert!(result.is_err(), "Should require 'ops' field");
}

#[test]
fn test_decoder_handles_empty_ops() {
    // Test that empty ops array is handled
    let decoder = ToonDecoder::new();

    let json = r#"{"ops": []}"#;

    let result = decoder.decode_ops(json);
    assert!(result.is_ok(), "Empty ops should be valid");
    assert_eq!(result.unwrap().len(), 0);
}

#[test]
fn test_decoder_requires_type_field() {
    // Test that missing "type" field causes error
    let decoder = ToonDecoder::new();

    let json = r#"{"ops": [{"query": "test"}]}"#;

    let result = decoder.decode_ops(json);
    assert!(result.is_err(), "Should require 'type' field");
}

#[test]
fn test_decoder_validates_retrieve_fields() {
    // Test that Retrieve requires both query and k
    let decoder = ToonDecoder::new();

    let missing_k = r#"{"ops": [{"type": "retrieve", "query": "test"}]}"#;
    assert!(decoder.decode_ops(missing_k).is_err(), "Should require 'k' field");

    let missing_query = r#"{"ops": [{"type": "retrieve", "k": 5}]}"#;
    assert!(decoder.decode_ops(missing_query).is_err(), "Should require 'query' field");
}

#[test]
fn test_decoder_validates_fold_context_fields() {
    // Test that FoldContext requires context_ids
    let decoder = ToonDecoder::new();

    let missing_ids = r#"{"ops": [{"type": "fold_context"}]}"#;
    assert!(decoder.decode_ops(missing_ids).is_err(), "Should require 'context_ids' field");

    let valid = r#"{"ops": [{"type": "fold_context", "context_ids": ["id1"]}]}"#;
    assert!(decoder.decode_ops(valid).is_ok(), "Valid fold_context should parse");
}

#[test]
fn test_decoder_is_deterministic() {
    // Test that multiple parses of same JSON produce identical results
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
            ToonInstr::Retrieve {
                query: q1,
                k: k1,
            },
            ToonInstr::Retrieve {
                query: q2,
                k: k2,
            },
        ) => {
            assert_eq!(q1, q2);
            assert_eq!(k1, k2);
        }
        _ => panic!("Instructions should match"),
    }
}
