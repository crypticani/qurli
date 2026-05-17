use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::auth::{is_secret_name, mask_secret};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeVariable {
    pub key: String,
    pub value: String,
    pub secret: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtractionRule {
    pub variable_name: String,
    pub json_path: String,
    pub secret: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubstitutionResult {
    pub value: String,
    pub unresolved: Vec<String>,
}

pub fn substitute(
    input: &str,
    variables: &HashMap<String, RuntimeVariable>,
    mask_secrets: bool,
) -> SubstitutionResult {
    let mut output = String::with_capacity(input.len());
    let mut unresolved = Vec::new();
    let mut rest = input;

    while let Some(start) = rest.find("{{") {
        let (prefix, after_start) = rest.split_at(start);
        output.push_str(prefix);
        let after_start = &after_start[2..];

        if let Some(end) = after_start.find("}}") {
            let key = after_start[..end].trim();
            if let Some(variable) = variables.get(key) {
                if mask_secrets && variable.secret {
                    output.push_str(&mask_secret(&variable.value));
                } else {
                    output.push_str(&variable.value);
                }
            } else {
                unresolved.push(key.to_string());
                output.push_str("{{");
                output.push_str(key);
                output.push_str("}}");
            }
            rest = &after_start[end + 2..];
        } else {
            output.push_str("{{");
            output.push_str(after_start);
            rest = "";
        }
    }

    output.push_str(rest);
    unresolved.sort();
    unresolved.dedup();

    SubstitutionResult {
        value: output,
        unresolved,
    }
}

pub fn parse_extraction_rules(input: &str) -> Vec<ExtractionRule> {
    input
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }

            let (name, path) = line.split_once('=')?;
            let variable_name = name.trim().to_string();
            let json_path = path.trim().to_string();

            if variable_name.is_empty() || json_path.is_empty() {
                return None;
            }

            Some(ExtractionRule {
                secret: is_secret_name(&variable_name),
                variable_name,
                json_path,
            })
        })
        .collect()
}

pub fn apply_extraction_rules(
    body: &str,
    rules: &[ExtractionRule],
    variables: &mut HashMap<String, RuntimeVariable>,
) -> Result<usize, String> {
    if rules.is_empty() {
        return Ok(0);
    }

    let json: Value =
        serde_json::from_str(body).map_err(|err| format!("Response is not valid JSON: {err}"))?;
    let mut applied = 0;

    for rule in rules {
        if let Some(value) = extract_json_path(&json, &rule.json_path) {
            variables.insert(
                rule.variable_name.clone(),
                RuntimeVariable {
                    key: rule.variable_name.clone(),
                    value: json_value_to_string(value),
                    secret: rule.secret || is_secret_name(&rule.variable_name),
                },
            );
            applied += 1;
        }
    }

    Ok(applied)
}

pub fn extract_json_path<'a>(json: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = json;
    let path = path.trim().strip_prefix("$.")?;

    for segment in path.split('.') {
        if segment.is_empty() {
            return None;
        }

        let mut name = segment;
        let mut indexes = Vec::new();

        while let Some(start) = name.find('[') {
            let end = name[start + 1..].find(']')? + start + 1;
            let index = name[start + 1..end].parse::<usize>().ok()?;
            indexes.push(index);
            name = &name[..start];
        }

        if !name.is_empty() {
            current = current.get(name)?;
        }

        for index in indexes {
            current = current.get(index)?;
        }
    }

    Some(current)
}

pub fn json_value_to_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Number(value) => value.to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Null => "null".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_known_variables_and_reports_missing_ones() {
        let mut variables = HashMap::new();
        variables.insert(
            "token".to_string(),
            RuntimeVariable {
                key: "token".to_string(),
                value: "abc123".to_string(),
                secret: true,
            },
        );

        let result = substitute("Bearer {{ token }} {{missing}}", &variables, false);
        assert_eq!(result.value, "Bearer abc123 {{missing}}");
        assert_eq!(result.unresolved, vec!["missing"]);

        let masked = substitute("Bearer {{token}}", &variables, true);
        assert_eq!(masked.value, "Bearer ********");
    }

    #[test]
    fn parses_extraction_rules() {
        let rules =
            parse_extraction_rules("token = $.access_token\n# comment\nuser_id = $.user.id");
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].variable_name, "token");
        assert!(rules[0].secret);
        assert_eq!(rules[1].json_path, "$.user.id");
    }

    #[test]
    fn extracts_simple_dot_paths_and_array_indexes() {
        let json: Value = serde_json::json!({
            "access_token": "abc123",
            "user": { "id": 42 },
            "items": [{ "id": "first" }]
        });

        assert_eq!(
            extract_json_path(&json, "$.access_token").map(json_value_to_string),
            Some("abc123".to_string())
        );
        assert_eq!(
            extract_json_path(&json, "$.user.id").map(json_value_to_string),
            Some("42".to_string())
        );
        assert_eq!(
            extract_json_path(&json, "$.items[0].id").map(json_value_to_string),
            Some("first".to_string())
        );
        assert!(extract_json_path(&json, "$.items[1].id").is_none());
    }

    #[test]
    fn applies_extraction_rules_to_runtime_store() {
        let rules = parse_extraction_rules("token = $.access_token\nuser_id = $.user.id");
        let mut variables = HashMap::new();
        let count = apply_extraction_rules(
            r#"{"access_token":"abc123","user":{"id":42}}"#,
            &rules,
            &mut variables,
        )
        .unwrap();

        assert_eq!(count, 2);
        assert_eq!(variables["token"].value, "abc123");
        assert!(variables["token"].secret);
        assert_eq!(variables["user_id"].value, "42");
        assert!(!variables["user_id"].secret);
    }
}
