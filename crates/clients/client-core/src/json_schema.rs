//! The ONE sanctioned opaque-JSON boundary for request structs.
//!
//! Request/response structs must never carry bare `serde_json::Value` fields
//! (grab-bags defeat the type system). JSON *Schema* payloads are the sole
//! legitimately-dynamic case — a schema is arbitrary JSON by nature — so they
//! ride behind this validating newtype instead: construction checks the shape,
//! serialization is transparent, and every use site names its intent.

use serde::{Deserialize, Serialize};

/// A JSON Schema document (for structured-output / function-parameter
/// declarations). Constructed via [`JsonSchemaDef::new`], which rejects
/// payloads that cannot be a schema.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JsonSchemaDef(serde_json::Value);

impl JsonSchemaDef {
    /// Wrap a schema document. Per the JSON Schema spec a schema is an object
    /// (or a bare boolean); anything else is rejected here, client-side.
    pub fn new(schema: serde_json::Value) -> Result<Self, String> {
        if schema.is_object() || schema.is_boolean() {
            Ok(Self(schema))
        } else {
            Err(format!("a JSON Schema must be an object or boolean, got: {schema}"))
        }
    }

    pub fn as_value(&self) -> &serde_json::Value {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn accepts_objects_and_booleans_rejects_the_rest() {
        assert!(JsonSchemaDef::new(json!({ "type": "object" })).is_ok());
        assert!(JsonSchemaDef::new(json!(true)).is_ok());
        assert!(JsonSchemaDef::new(json!("nope")).is_err());
        assert!(JsonSchemaDef::new(json!(42)).is_err());
        assert!(JsonSchemaDef::new(json!(["a"])).is_err());
    }

    #[test]
    fn serializes_transparently() {
        let schema = JsonSchemaDef::new(json!({ "type": "string" })).unwrap();
        assert_eq!(serde_json::to_string(&schema).unwrap(), r#"{"type":"string"}"#);
    }
}
