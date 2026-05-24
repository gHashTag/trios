//! # CR-CHAT-06 — Tool argument schema guard (Wave-68 Lane B)
//!
//! AGENT SAFETY — tool arguments must match their JSON-schema, R-CHAT-7.
//!
//! Without schema validation, an LLM agent can pass:
//!
//! * **Type confusion** — string where integer expected, bypassing bounds.
//! * **Missing required fields** — tool runs with incomplete args.
//! * **Unknown fields** — extra fields may trigger unintended code paths.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Argument type matches schema type.
//! 2. All required fields are present.
//! 3. No unknown fields (strict mode).
//! 4. String length <= `TASG_MAX_STRING_LEN`.
//! 5. Number of properties <= `TASG_MAX_PROPS`.
//! 6. Schema must have at least 1 property.
//!
//! Tests **TASG-01..10**. Error enum [`SchemaError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * TOOL-ARG-SCHEMA`

#![forbid(unsafe_code)]

/// Maximum string length per argument.
pub const TASG_MAX_STRING_LEN: usize = 4096;

/// Maximum number of properties per tool call.
pub const TASG_MAX_PROPS: usize = 32;

/// Allowed value types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaType {
    /// String type.
    StringType,
    /// Integer type.
    IntType,
    /// Boolean type.
    BoolType,
}

/// A schema property definition.
#[derive(Debug, Clone)]
pub struct PropertyDef {
    /// Property name.
    pub name: String,
    /// Expected type.
    pub prop_type: SchemaType,
    /// Is this property required?
    pub required: bool,
}

/// A provided argument value.
#[derive(Debug, Clone)]
pub enum SchemaArgValue {
    /// String value.
    StringVal(String),
    /// Integer value.
    IntVal(i64),
    /// Boolean value.
    BoolVal(bool),
}

/// All ways schema validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SchemaError {
    /// Type mismatch for property.
    TypeMismatch(String),
    /// Missing required property.
    MissingRequired(String),
    /// Unknown property not in schema.
    UnknownField(String),
    /// String too long.
    StringTooLong(String),
    /// Too many properties.
    TooManyProperties,
    /// Empty schema.
    EmptySchema,
}

/// `[VERIFIED]` Validate tool arguments against a schema.
pub fn validate_tool_args(
    schema: &[PropertyDef],
    args: &[(&str, SchemaArgValue)],
) -> Result<(), SchemaError> {
    if schema.is_empty() {
        return Err(SchemaError::EmptySchema);
    }
    if args.len() > TASG_MAX_PROPS {
        return Err(SchemaError::TooManyProperties);
    }
    let schema_map: std::collections::BTreeMap<&str, &PropertyDef> = schema
        .iter()
        .map(|p| (p.name.as_str(), p))
        .collect();
    let mut provided: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for (name, val) in args {
        match schema_map.get(name) {
            None => return Err(SchemaError::UnknownField(name.to_string())),
            Some(def) => {
                let ok = match (&def.prop_type, val) {
                    (SchemaType::StringType, SchemaArgValue::StringVal(s)) => {
                        if s.len() > TASG_MAX_STRING_LEN {
                            return Err(SchemaError::StringTooLong(name.to_string()));
                        }
                        true
                    }
                    (SchemaType::IntType, SchemaArgValue::IntVal(_)) => true,
                    (SchemaType::BoolType, SchemaArgValue::BoolVal(_)) => true,
                    _ => false,
                };
                if !ok {
                    return Err(SchemaError::TypeMismatch(name.to_string()));
                }
            }
        }
        provided.insert(name);
    }
    for def in schema {
        if def.required && !provided.contains(def.name.as_str()) {
            return Err(SchemaError::MissingRequired(def.name.clone()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_schema() -> Vec<PropertyDef> {
        vec![
            PropertyDef { name: "path".into(), prop_type: SchemaType::StringType, required: true },
            PropertyDef { name: "count".into(), prop_type: SchemaType::IntType, required: true },
            PropertyDef { name: "verbose".into(), prop_type: SchemaType::BoolType, required: false },
        ]
    }

    fn valid_args() -> Vec<(&'static str, SchemaArgValue)> {
        vec![
            ("path", SchemaArgValue::StringVal("/tmp/file".into())),
            ("count", SchemaArgValue::IntVal(42)),
        ]
    }

    /// **TASG-01** — type mismatch rejected.
    #[test]
    fn tasg_01_type_mismatch_rejected() {
        let args = vec![
            ("path", SchemaArgValue::IntVal(42)),
            ("count", SchemaArgValue::IntVal(1)),
        ];
        assert_eq!(
            validate_tool_args(&simple_schema(), &args),
            Err(SchemaError::TypeMismatch("path".into()))
        );
    }

    /// **TASG-02** — missing required rejected.
    #[test]
    fn tasg_02_missing_required_rejected() {
        let args = vec![("verbose", SchemaArgValue::BoolVal(true))];
        assert_eq!(
            validate_tool_args(&simple_schema(), &args),
            Err(SchemaError::MissingRequired("path".into()))
        );
    }

    /// **TASG-03** — unknown field rejected.
    #[test]
    fn tasg_03_unknown_field_rejected() {
        let args = vec![
            ("path", SchemaArgValue::StringVal("/x".into())),
            ("count", SchemaArgValue::IntVal(1)),
            ("evil", SchemaArgValue::StringVal("payload".into())),
        ];
        assert_eq!(
            validate_tool_args(&simple_schema(), &args),
            Err(SchemaError::UnknownField("evil".into()))
        );
    }

    /// **TASG-04** — string too long rejected.
    #[test]
    fn tasg_04_string_too_long_rejected() {
        let long = "x".repeat(TASG_MAX_STRING_LEN + 1);
        let args = vec![("path", SchemaArgValue::StringVal(long))];
        assert_eq!(
            validate_tool_args(&simple_schema(), &args),
            Err(SchemaError::StringTooLong("path".into()))
        );
    }

    /// **TASG-05** — too many properties rejected.
    #[test]
    fn tasg_05_too_many_props_rejected() {
        let schema = vec![
            PropertyDef { name: "a".into(), prop_type: SchemaType::IntType, required: true },
        ];
        let args: Vec<(&str, SchemaArgValue)> = (0..=TASG_MAX_PROPS)
            .map(|i| {
                let mut name = String::new();
                name.push(char::from(b'a' + (i % 26) as u8));
                (Box::leak(name.into_boxed_str()) as &str, SchemaArgValue::IntVal(i as i64))
            })
            .collect();
        assert_eq!(
            validate_tool_args(&schema, &args),
            Err(SchemaError::TooManyProperties)
        );
    }

    /// **TASG-06** — empty schema rejected.
    #[test]
    fn tasg_06_empty_schema_rejected() {
        assert_eq!(
            validate_tool_args(&[], &[]),
            Err(SchemaError::EmptySchema)
        );
    }

    /// **TASG-07** — valid args accepted.
    #[test]
    fn tasg_07_valid_accepted() {
        assert_eq!(validate_tool_args(&simple_schema(), &valid_args()), Ok(()));
    }

    /// **TASG-08** — all three args accepted.
    #[test]
    fn tasg_08_all_args_accepted() {
        let args = vec![
            ("path", SchemaArgValue::StringVal("/tmp".into())),
            ("count", SchemaArgValue::IntVal(10)),
            ("verbose", SchemaArgValue::BoolVal(true)),
        ];
        assert_eq!(validate_tool_args(&simple_schema(), &args), Ok(()));
    }

    /// **TASG-09** — only required args accepted.
    #[test]
    fn tasg_09_required_only_accepted() {
        assert_eq!(validate_tool_args(&simple_schema(), &valid_args()), Ok(()));
    }

    /// **TASG-10** — max string length accepted.
    #[test]
    fn tasg_10_max_string_accepted() {
        let s = "x".repeat(TASG_MAX_STRING_LEN);
        let args = vec![
            ("path", SchemaArgValue::StringVal(s)),
            ("count", SchemaArgValue::IntVal(1)),
        ];
        assert_eq!(validate_tool_args(&simple_schema(), &args), Ok(()));
    }
}
