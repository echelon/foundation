use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, ensure, Context, Result};
use serde::Deserialize;
use serde_json::Value;

const BUILTIN: &[u8] = include_bytes!("../schemas/v1.json");

/// The primitive definitions and validation rules shared by the mapper and connectors.
pub struct Registry {
    document: Value,
    index: SchemaDocument,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SchemaDocument {
    #[serde(rename = "$schema")]
    dialect: String,
    #[serde(rename = "$id")]
    id: String,
    #[serde(rename = "$defs")]
    definitions: BTreeMap<String, Schema>,
    #[serde(rename = "title")]
    _title: Option<String>,
    #[serde(rename = "description")]
    _description: Option<String>,
}

#[derive(Clone, Copy, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
enum Kind {
    Object,
    Array,
    String,
    Integer,
    Number,
    Boolean,
    Null,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Types {
    One(Kind),
    Multiple(Vec<Kind>),
}

impl Types {
    fn contains(&self, kind: Kind) -> bool {
        match self {
            Self::One(value) => *value == kind,
            Self::Multiple(values) => values.contains(&kind),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct Schema {
    #[serde(rename = "type")]
    types: Types,
    #[serde(default)]
    pub(crate) properties: BTreeMap<String, Schema>,
    #[serde(default)]
    pub(crate) required: BTreeSet<String>,
    additional_properties: Option<bool>,
    pub(crate) items: Option<Box<Schema>>,
    #[serde(rename = "enum")]
    values: Option<Vec<Value>>,
    min_length: Option<usize>,
    max_length: Option<usize>,
    min_items: Option<usize>,
    max_items: Option<usize>,
    minimum: Option<f64>,
    maximum: Option<f64>,
    #[serde(rename = "description")]
    _description: Option<String>,
}

impl Registry {
    pub fn builtin_json() -> &'static [u8] {
        BUILTIN
    }

    pub fn builtin() -> Result<Self> {
        Self::from_slice(BUILTIN)
    }

    pub fn from_slice(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() <= 256 * 1024, "schema registry exceeds 256 KiB");
        let document = serde_json::from_slice(bytes).context("read schema JSON")?;
        let index: SchemaDocument =
            serde_json::from_slice(bytes).context("read schema definitions")?;
        ensure!(
            index.dialect == "https://json-schema.org/draft/2020-12/schema"
                && index.id.starts_with("urn:tokencraft:primitives:"),
            "expected a versioned TokenCraft primitive schema registry"
        );
        ensure!(!index.definitions.is_empty(), "no primitive definitions");
        for (name, schema) in &index.definitions {
            let pair = if let Some(primitive) = name.strip_suffix("_input") {
                format!("{primitive}_result")
            } else if let Some(primitive) = name.strip_suffix("_result") {
                format!("{primitive}_input")
            } else {
                bail!("definition {name} requires an _input or _result suffix");
            };
            ensure!(
                index.definitions.contains_key(&pair) && schema.types.contains(Kind::Object),
                "definition {name} needs a matching object definition {pair}"
            );
            schema
                .check()
                .with_context(|| format!("invalid definition {name}"))?;
        }
        Ok(Self { document, index })
    }

    pub fn id(&self) -> &str {
        &self.index.id
    }

    pub fn document(&self) -> &Value {
        &self.document
    }

    pub(crate) fn definition(&self, primitive: &str, result: bool) -> Result<&Schema> {
        let suffix = if result { "result" } else { "input" };
        self.index
            .definitions
            .get(&format!("{primitive}_{suffix}"))
            .with_context(|| format!("unknown primitive {primitive}"))
    }

    pub fn validate_input(&self, primitive: &str, input: &Value) -> Result<()> {
        self.definition(primitive, false)?.validate(input, "$")
    }

    pub fn primitive(&self, primitive: &str) -> Result<Value> {
        self.definition(primitive, false)?;
        let defs = &self.document["$defs"];
        Ok(serde_json::json!({
            "schema": self.id(),
            "primitive": primitive,
            "input": defs[format!("{primitive}_input")],
            "result": defs[format!("{primitive}_result")]
        }))
    }
}

impl Schema {
    fn check(&self) -> Result<()> {
        if let Types::Multiple(kinds) = &self.types {
            let distinct: BTreeSet<_> = kinds.iter().collect();
            ensure!(
                !kinds.is_empty() && distinct.len() == kinds.len(),
                "invalid type list"
            );
        }
        if self.types.contains(Kind::Object) {
            ensure!(
                self.additional_properties == Some(false),
                "object fields must be explicit"
            );
            ensure!(
                self.required
                    .iter()
                    .all(|name| self.properties.contains_key(name)),
                "required field is missing a definition"
            );
        }
        if self.types.contains(Kind::Array) {
            ensure!(self.items.is_some(), "arrays require items");
        }
        for child in self.properties.values().chain(self.items.as_deref()) {
            child.check()?;
        }
        if let Some(values) = &self.values {
            ensure!(!values.is_empty(), "empty enum");
        }
        ensure!(
            self.min_length
                .zip(self.max_length)
                .is_none_or(|(min, max)| min <= max)
                && self
                    .min_items
                    .zip(self.max_items)
                    .is_none_or(|(min, max)| min <= max)
                && self
                    .minimum
                    .zip(self.maximum)
                    .is_none_or(|(min, max)| min <= max),
            "invalid schema bounds"
        );
        Ok(())
    }

    pub(crate) fn field(&self, path: &[String]) -> Option<&Schema> {
        let mut field = self;
        for part in path {
            field = if let Some(item) = field.items.as_deref() {
                part.parse::<usize>()
                    .ok()
                    .filter(|index| index.to_string() == *part)?;
                item
            } else {
                field.properties.get(part)?
            };
        }
        Some(field)
    }

    pub(crate) fn validate(&self, value: &Value, path: &str) -> Result<()> {
        let kind_ok = match value {
            Value::Object(_) => self.types.contains(Kind::Object),
            Value::Array(_) => self.types.contains(Kind::Array),
            Value::String(_) => self.types.contains(Kind::String),
            Value::Number(value) => {
                self.types.contains(Kind::Number)
                    || (self.types.contains(Kind::Integer)
                        && value.as_f64().is_some_and(|number| number.fract() == 0.0))
            }
            Value::Bool(_) => self.types.contains(Kind::Boolean),
            Value::Null => self.types.contains(Kind::Null),
        };
        ensure!(kind_ok, "{path}: incompatible JSON type");
        if let Some(allowed) = &self.values {
            ensure!(
                allowed.contains(value),
                "{path}: value outside the schema enum"
            );
        }
        match value {
            Value::Object(fields) => {
                for required in &self.required {
                    ensure!(fields.contains_key(required), "{path}: missing {required}");
                }
                for (key, field) in fields {
                    self.properties
                        .get(key)
                        .with_context(|| format!("{path}: unknown field {key}"))?
                        .validate(field, &format!("{path}/{}", escape(key)))?;
                }
            }
            Value::Array(values) => {
                bounds(values.len(), self.min_items, self.max_items, path)?;
                let item = self
                    .items
                    .as_deref()
                    .context("array is missing an item schema")?;
                for (index, value) in values.iter().enumerate() {
                    item.validate(value, &format!("{path}/{index}"))?;
                }
            }
            Value::String(value) => bounds(
                value.chars().count(),
                self.min_length,
                self.max_length,
                path,
            )?,
            Value::Number(value) => bounds(
                value.as_f64().context("number outside finite range")?,
                self.minimum,
                self.maximum,
                path,
            )?,
            _ => {}
        }
        Ok(())
    }
}

fn bounds<T: PartialOrd>(value: T, min: Option<T>, max: Option<T>, path: &str) -> Result<()> {
    ensure!(
        min.is_none_or(|limit| value >= limit) && max.is_none_or(|limit| value <= limit),
        "{path}: outside the schema bounds"
    );
    Ok(())
}

pub(crate) fn escape(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}
