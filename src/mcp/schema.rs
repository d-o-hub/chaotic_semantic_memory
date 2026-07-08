//! JSON Schema definitions for MCP tool inputs (ADR-0067)

use serde_json::{Value, json};

/// Schema for memory_inject tool.
pub fn inject_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "concept_id": {
                "type": "string",
                "description": "Unique identifier for the concept"
            },
            "metadata": {
                "type": "object",
                "description": "Optional metadata key-value pairs"
            }
        },
        "required": ["concept_id"]
    })
}

/// Schema for memory_inject_text tool.
pub fn inject_text_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "concept_id": {
                "type": "string",
                "description": "Unique identifier for the concept"
            },
            "text": {
                "type": "string",
                "description": "Text to encode as concept vector"
            },
            "metadata": {
                "type": "object",
                "description": "Optional metadata key-value pairs"
            }
        },
        "required": ["concept_id", "text"]
    })
}

/// Schema for memory_probe tool.
pub fn probe_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "concept_id": {
                "type": "string",
                "description": "ID of concept to use as query vector"
            },
            "top_k": {
                "type": "integer",
                "description": "Number of results to return",
                "default": 10
            }
        },
        "required": ["concept_id"]
    })
}

/// Schema for memory_probe_text tool.
pub fn probe_text_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "text": {
                "type": "string",
                "description": "Query text to encode and search"
            },
            "top_k": {
                "type": "integer",
                "description": "Number of results to return",
                "default": 10
            }
        },
        "required": ["text"]
    })
}

/// Schema for memory_probe_filtered tool.
pub fn probe_filtered_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "text": {
                "type": "string",
                "description": "Query text"
            },
            "top_k": {
                "type": "integer",
                "description": "Number of results",
                "default": 10
            },
            "filter": {
                "type": "object",
                "description": "Metadata filter criteria"
            }
        },
        "required": ["text"]
    })
}

/// Schema for memory_get tool.
pub fn get_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "concept_id": {
                "type": "string",
                "description": "Concept ID to retrieve"
            }
        },
        "required": ["concept_id"]
    })
}

/// Schema for memory_delete tool.
pub fn delete_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "concept_id": {
                "type": "string",
                "description": "Concept ID to delete"
            }
        },
        "required": ["concept_id"]
    })
}

/// Schema for memory_associate tool.
pub fn associate_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "from_id": {
                "type": "string",
                "description": "Source concept ID"
            },
            "to_id": {
                "type": "string",
                "description": "Target concept ID"
            },
            "strength": {
                "type": "number",
                "description": "Association strength (0.0-1.0)",
                "default": 0.5
            }
        },
        "required": ["from_id", "to_id"]
    })
}

/// Schema for memory_traverse tool.
pub fn traverse_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "start_id": {
                "type": "string",
                "description": "Starting concept ID"
            },
            "depth": {
                "type": "integer",
                "description": "Maximum traversal depth",
                "default": 3
            },
            "min_strength": {
                "type": "number",
                "description": "Minimum association strength",
                "default": 0.0
            }
        },
        "required": ["start_id"]
    })
}

/// Schema for memory_shortest_path tool.
pub fn shortest_path_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "from_id": {
                "type": "string",
                "description": "Starting concept ID"
            },
            "to_id": {
                "type": "string",
                "description": "Target concept ID"
            },
            "weighted": {
                "type": "boolean",
                "description": "Use association strength as edge weight",
                "default": true
            }
        },
        "required": ["from_id", "to_id"]
    })
}

/// Schema for memory_stats tool.
pub fn stats_schema() -> Value {
    json!({
        "type": "object",
        "properties": {}
    })
}

/// Schema for memory_export tool.
pub fn export_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "format": {
                "type": "string",
                "enum": ["json", "binary"],
                "default": "json"
            }
        }
    })
}

/// Schema for memory_list_gaps tool.
pub fn list_gaps_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "min_attempts": {
                "type": "integer",
                "description": "Minimum number of failed retrieval attempts",
                "default": 1
            }
        }
    })
}

#[cfg(all(test, feature = "mcp"))]
mod tests {
    use super::*;
    use serde_json::json;

    fn assert_is_object_schema(schema: &Value, name: &str) {
        assert_eq!(schema["type"], "object", "{name} should have type object");
        assert!(
            schema["properties"].is_object(),
            "{name} should have properties object"
        );
    }

    #[test]
    fn test_inject_schema() {
        let s = inject_schema();
        assert_is_object_schema(&s, "inject_schema");
        assert!(s["properties"]["concept_id"].is_object());
        assert!(s["properties"]["metadata"].is_object());
        assert_eq!(s["required"], json!(["concept_id"]));
    }

    #[test]
    fn test_inject_text_schema() {
        let s = inject_text_schema();
        assert_is_object_schema(&s, "inject_text_schema");
        assert!(s["properties"]["concept_id"].is_object());
        assert!(s["properties"]["text"].is_object());
        assert!(s["properties"]["metadata"].is_object());
        assert_eq!(s["required"], json!(["concept_id", "text"]));
    }

    #[test]
    fn test_probe_schema() {
        let s = probe_schema();
        assert_is_object_schema(&s, "probe_schema");
        assert!(s["properties"]["concept_id"].is_object());
        assert!(s["properties"]["top_k"].is_object());
        assert_eq!(s["required"], json!(["concept_id"]));
    }

    #[test]
    fn test_probe_text_schema() {
        let s = probe_text_schema();
        assert_is_object_schema(&s, "probe_text_schema");
        assert!(s["properties"]["text"].is_object());
        assert!(s["properties"]["top_k"].is_object());
        assert_eq!(s["required"], json!(["text"]));
    }

    #[test]
    fn test_probe_filtered_schema() {
        let s = probe_filtered_schema();
        assert_is_object_schema(&s, "probe_filtered_schema");
        assert!(s["properties"]["text"].is_object());
        assert!(s["properties"]["top_k"].is_object());
        assert!(s["properties"]["filter"].is_object());
        assert_eq!(s["required"], json!(["text"]));
    }

    #[test]
    fn test_get_schema() {
        let s = get_schema();
        assert_is_object_schema(&s, "get_schema");
        assert!(s["properties"]["concept_id"].is_object());
        assert_eq!(s["required"], json!(["concept_id"]));
    }

    #[test]
    fn test_delete_schema() {
        let s = delete_schema();
        assert_is_object_schema(&s, "delete_schema");
        assert!(s["properties"]["concept_id"].is_object());
        assert_eq!(s["required"], json!(["concept_id"]));
    }

    #[test]
    fn test_associate_schema() {
        let s = associate_schema();
        assert_is_object_schema(&s, "associate_schema");
        assert!(s["properties"]["from_id"].is_object());
        assert!(s["properties"]["to_id"].is_object());
        assert!(s["properties"]["strength"].is_object());
        assert_eq!(s["required"], json!(["from_id", "to_id"]));
    }

    #[test]
    fn test_traverse_schema() {
        let s = traverse_schema();
        assert_is_object_schema(&s, "traverse_schema");
        assert!(s["properties"]["start_id"].is_object());
        assert!(s["properties"]["depth"].is_object());
        assert!(s["properties"]["min_strength"].is_object());
        assert_eq!(s["required"], json!(["start_id"]));
    }

    #[test]
    fn test_shortest_path_schema() {
        let s = shortest_path_schema();
        assert_is_object_schema(&s, "shortest_path_schema");
        assert!(s["properties"]["from_id"].is_object());
        assert!(s["properties"]["to_id"].is_object());
        assert!(s["properties"]["weighted"].is_object());
        assert_eq!(s["required"], json!(["from_id", "to_id"]));
    }

    #[test]
    fn test_stats_schema() {
        let s = stats_schema();
        assert_is_object_schema(&s, "stats_schema");
    }

    #[test]
    fn test_export_schema() {
        let s = export_schema();
        assert_is_object_schema(&s, "export_schema");
        let format = &s["properties"]["format"];
        assert!(format.is_object());
        assert_eq!(format["type"], "string");
        let enum_vals = format["enum"].as_array().unwrap();
        assert!(enum_vals.contains(&json!("json")));
        assert!(enum_vals.contains(&json!("binary")));
    }

    #[test]
    fn test_list_gaps_schema() {
        let s = list_gaps_schema();
        assert_is_object_schema(&s, "list_gaps_schema");
        assert!(s["properties"]["min_attempts"].is_object());
        assert_eq!(s["properties"]["min_attempts"]["type"], "integer");
        assert_eq!(s["properties"]["min_attempts"]["default"], 1);
    }

    #[test]
    fn test_all_schemas_have_type_object() {
        let schemas = vec![
            ("inject", inject_schema()),
            ("inject_text", inject_text_schema()),
            ("probe", probe_schema()),
            ("probe_text", probe_text_schema()),
            ("probe_filtered", probe_filtered_schema()),
            ("get", get_schema()),
            ("delete", delete_schema()),
            ("associate", associate_schema()),
            ("traverse", traverse_schema()),
            ("shortest_path", shortest_path_schema()),
            ("stats", stats_schema()),
            ("export", export_schema()),
            ("list_gaps", list_gaps_schema()),
        ];
        for (name, schema) in &schemas {
            assert_eq!(
                schema["type"], "object",
                "{name} schema must have type 'object'"
            );
        }
    }

    #[test]
    fn test_inject_schema_description_fields() {
        let s = inject_schema();
        assert!(
            s["properties"]["concept_id"]["description"]
                .as_str()
                .is_some()
        );
        assert!(
            s["properties"]["metadata"]["description"]
                .as_str()
                .is_some()
        );
    }
}
