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
            },
            "min_score": {
                "type": "number",
                "description": "Minimum confidence threshold (0.0-1.0)",
                "default": 0.0
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
