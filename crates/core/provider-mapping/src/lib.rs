mod schema;

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub use schema::Registry;
use schema::{escape, Schema};

const MAX_FILE_BYTES: usize = 256 * 1024;
const MAX_PREVIEW_BYTES: usize = 512 * 1024;

#[derive(Clone, Copy, Deserialize, Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Draft,
    Approved,
}

#[derive(Clone, Copy)]
pub enum Direction {
    Request,
    Response,
}

/// Model-specific JSON field translations. Files contain data and never choose transport.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Mapping {
    pub version: u8,
    pub schema: String,
    pub primitive: String,
    pub provider: String,
    pub model: String,
    pub revision: String,
    pub status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Evidence>,
    pub rules: MappingRules,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub reference: String,
    pub sha256: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MappingRules {
    pub request: Vec<Rule>,
    pub response: Vec<Rule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ignore_request: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    to: String,
    #[serde(default, skip_serializing_if = "is_false")]
    optional: bool,
    value: Operation,
}

fn is_false(value: &bool) -> bool {
    !value
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum Operation {
    Copy {
        from: String,
    },
    Literal {
        value: Value,
    },
    Lookup {
        from: String,
        values: BTreeMap<String, Value>,
    },
    Multiply {
        from: String,
        factor: i32,
    },
    Each {
        from: String,
        fields: Vec<Rule>,
    },
}

impl Operation {
    fn source(&self) -> Option<&str> {
        match self {
            Self::Copy { from }
            | Self::Lookup { from, .. }
            | Self::Multiply { from, .. }
            | Self::Each { from, .. } => Some(from),
            Self::Literal { .. } => None,
        }
    }

    fn evaluate(
        &self,
        input: &Value,
        optional: bool,
        remaining: &mut usize,
    ) -> Result<Option<Value>> {
        let Some(from) = self.source() else {
            let Self::Literal { value } = self else {
                unreachable!()
            };
            return Ok(Some(copy(value, remaining)?));
        };
        let Some(value) = input.pointer(from) else {
            ensure!(optional, "source {from:?} is missing");
            return Ok(None);
        };
        Ok(Some(match self {
            Self::Copy { .. } => copy(value, remaining)?,
            Self::Lookup { values, .. } => {
                let key = value.as_str().context("lookup requires a string")?;
                let result = values
                    .get(key)
                    .context("no lookup entry for the supplied value")?;
                copy(result, remaining)?
            }
            Self::Multiply { factor, .. } => {
                reserve(20, remaining)?;
                let number = value
                    .as_i64()
                    .context("multiply requires a signed integer")?;
                number
                    .checked_mul(i64::from(*factor))
                    .context("multiply overflow")?
                    .into()
            }
            Self::Each { fields, .. } => {
                let values = value.as_array().context("each requires an array")?;
                ensure!(values.len() <= 64, "each supports up to 64 items");
                reserve(values.len() + 2, remaining)?;
                Value::Array(
                    values
                        .iter()
                        .map(|value| project(fields, value, remaining))
                        .collect::<Result<_>>()?,
                )
            }
            Self::Literal { .. } => unreachable!(),
        }))
    }
}

impl Mapping {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() <= MAX_FILE_BYTES, "mapping exceeds 256 KiB");
        serde_json::from_slice(bytes).context("decode mapping JSON")
    }

    pub fn validate(&self, registry: &Registry) -> Result<()> {
        ensure!(self.version == 1, "expected mapping version 1");
        ensure!(
            self.schema == registry.id(),
            "mapping references a different schema ID"
        );
        ensure!(
            slug(&self.provider) && slug(&self.primitive) && slug(&self.revision),
            "provider, primitive and revision must use lowercase IDs"
        );
        ensure!(
            (1..=160).contains(&self.model.len())
                && self
                    .model
                    .bytes()
                    .all(|byte| { byte.is_ascii_alphanumeric() || b"_-.:/".contains(&byte) }),
            "invalid upstream model ID"
        );
        let input = registry.definition(&self.primitive, false)?;
        let result = registry.definition(&self.primitive, true)?;
        ensure!(self.rules.notes.len() <= 32, "too many mapping notes");
        let complete = self.status == Status::Approved;
        validate_rules(&self.rules.request, Some(input), None, 0, complete)?;
        validate_rules(&self.rules.response, None, Some(result), 0, complete)?;
        let mut ignored = BTreeSet::new();
        for pointer in &self.rules.ignore_request {
            let path = parse_pointer(pointer)?;
            ensure!(
                !path.is_empty() && input.field(&path).is_some() && ignored.insert(pointer),
                "invalid or repeated ignored request field {pointer}"
            );
        }
        Ok(())
    }

    /// Try either direction with a sample. Drafts can be previewed.
    pub fn preview(
        &self,
        registry: &Registry,
        direction: Direction,
        input: &Value,
    ) -> Result<Value> {
        self.validate(registry)?;
        let mut remaining = MAX_PREVIEW_BYTES;
        let output = match direction {
            Direction::Request => {
                registry.validate_input(&self.primitive, input)?;
                check_coverage(input, "", &self.rules.request, &self.rules.ignore_request)?;
                project(&self.rules.request, input, &mut remaining)?
            }
            Direction::Response => {
                let result = project(&self.rules.response, input, &mut remaining)?;
                registry
                    .definition(&self.primitive, true)?
                    .validate(&result, "$")?;
                result
            }
        };
        ensure!(
            serde_json::to_vec(&output)?.len() <= MAX_PREVIEW_BYTES,
            "mapped payload exceeds 512 KiB"
        );
        Ok(output)
    }

    /// Apply the reviewed mapping selected for this exact provider and model.
    pub fn apply(
        &self,
        registry: &Registry,
        provider: &str,
        model: &str,
        direction: Direction,
        input: &Value,
    ) -> Result<Value> {
        ensure!(
            self.status == Status::Approved,
            "mapping must be approved to apply it"
        );
        ensure!(
            self.provider == provider && self.model == model,
            "mapping is for a different offer"
        );
        self.preview(registry, direction, input)
    }
}

fn slug(value: &str) -> bool {
    (1..=64).contains(&value.len())
        && value.as_bytes()[0].is_ascii_lowercase()
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
}

fn parse_pointer(pointer: &str) -> Result<Vec<String>> {
    ensure!(pointer.len() <= 256, "JSON pointer is too long");
    if pointer.is_empty() {
        return Ok(vec![]);
    }
    let raw = pointer
        .strip_prefix('/')
        .context("JSON pointer must start with /")?;
    raw.split('/')
        .map(|part| {
            ensure!(
                part.match_indices('~')
                    .all(|(index, _)| matches!(part.as_bytes().get(index + 1), Some(b'0' | b'1'))),
                "invalid JSON pointer escape"
            );
            Ok(part.replace("~1", "/").replace("~0", "~"))
        })
        .collect()
}

fn validate_rules(
    rules: &[Rule],
    source: Option<&Schema>,
    destination: Option<&Schema>,
    depth: usize,
    complete: bool,
) -> Result<()> {
    ensure!(
        rules.len() <= 64 && depth <= 4 && (!complete || !rules.is_empty()),
        "mapping rules exceed count or nesting limit"
    );
    let mut written: Vec<Vec<String>> = Vec::new();
    for rule in rules {
        let path = parse_pointer(&rule.to)?;
        ensure!(!path.is_empty(), "cannot replace the destination root");
        ensure!(
            written
                .iter()
                .all(|other| !other.starts_with(&path) && !path.starts_with(other)),
            "destination {} overlaps another rule",
            rule.to
        );
        let output = destination
            .map(|mut shape| {
                for part in &path {
                    shape = shape
                        .properties
                        .get(part)
                        .with_context(|| format!("unknown result field {}", rule.to))?;
                }
                Ok::<_, anyhow::Error>(shape)
            })
            .transpose()?;
        written.push(path);
        let field = if let Some(pointer) = rule.value.source() {
            let path = parse_pointer(pointer)?;
            source
                .map(|shape| {
                    shape
                        .field(&path)
                        .with_context(|| format!("unknown request field {pointer}"))
                })
                .transpose()?
        } else {
            None
        };
        match &rule.value {
            Operation::Literal { value } => {
                ensure!(!rule.optional, "literal cannot be optional");
                if let Some(output) = output {
                    output.validate(value, &rule.to)?;
                }
            }
            Operation::Lookup { values, .. } => {
                ensure!(
                    (1..=64).contains(&values.len()),
                    "lookup needs one to 64 values"
                );
                if let Some(output) = output {
                    for value in values.values() {
                        output.validate(value, &rule.to)?;
                    }
                }
            }
            Operation::Each { fields, .. } => {
                let source_items = field
                    .map(|schema| {
                        schema
                            .items
                            .as_deref()
                            .context("each requires an array source")
                    })
                    .transpose()?;
                let result_items = output
                    .map(|schema| {
                        schema
                            .items
                            .as_deref()
                            .context("each requires an array destination")
                    })
                    .transpose()?;
                validate_rules(fields, source_items, result_items, depth + 1, complete)?;
            }
            Operation::Copy { .. } | Operation::Multiply { .. } => {}
        }
    }
    if let Some(result) = destination.filter(|_| complete) {
        for required in &result.required {
            ensure!(
                written.iter().any(|path| &path[0] == required),
                "missing result mapping for {required}"
            );
        }
    }
    Ok(())
}

fn check_coverage(value: &Value, pointer: &str, rules: &[Rule], ignored: &[String]) -> Result<()> {
    if ignored.iter().any(|ignored| ignored == pointer) {
        return Ok(());
    }
    for rule in rules
        .iter()
        .filter(|rule| rule.value.source() == Some(pointer))
    {
        if let Operation::Each { fields, .. } = &rule.value {
            let items = value.as_array().context("each requires an array")?;
            for (index, item) in items.iter().enumerate() {
                check_coverage(item, "", fields, &[])
                    .with_context(|| format!("unmapped array item {pointer}/{index}"))?;
            }
        }
        return Ok(());
    }
    if let Some(fields) = value.as_object() {
        for (name, child) in fields {
            check_coverage(
                child,
                &format!("{pointer}/{}", escape(name)),
                rules,
                ignored,
            )?;
        }
    } else if let Some(items) = value.as_array() {
        ensure!(!items.is_empty(), "unmapped request field {pointer}");
        for (index, item) in items.iter().enumerate() {
            check_coverage(item, &format!("{pointer}/{index}"), rules, ignored)?;
        }
    } else {
        bail!("unmapped request field {pointer}");
    }
    Ok(())
}

fn reserve(size: usize, remaining: &mut usize) -> Result<()> {
    *remaining = remaining
        .checked_sub(size)
        .context("mapped payload exceeds 512 KiB")?;
    Ok(())
}

fn copy(value: &Value, remaining: &mut usize) -> Result<Value> {
    reserve(serde_json::to_vec(value)?.len(), remaining)?;
    Ok(value.clone())
}

fn project(rules: &[Rule], input: &Value, remaining: &mut usize) -> Result<Value> {
    reserve(2, remaining)?;
    let mut output = Map::new();
    for rule in rules {
        if let Some(value) = rule
            .value
            .evaluate(input, rule.optional, remaining)
            .with_context(|| format!("map {}", rule.to))?
        {
            let path = parse_pointer(&rule.to)?;
            reserve(path.iter().map(|part| part.len() + 4).sum(), remaining)?;
            let (field, ancestors) = path.split_last().context("empty destination")?;
            let mut object = &mut output;
            for part in ancestors {
                object = object
                    .entry(part)
                    .or_insert_with(|| Value::Object(Map::new()))
                    .as_object_mut()
                    .context("destination overlaps a leaf")?;
            }
            object.insert(field.to_owned(), value);
        }
    }
    Ok(Value::Object(output))
}
