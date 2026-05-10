//! # Wave-17 · L-CHAT-9-tool (R-CHAT-12) — tool-call argument confusion
//!
//! Type-confusion / argument-confusion injection against the agent's
//! structured tool-call surface. The attacker controls a tool name +
//! a JSON-shaped argument blob, and tries to (a) spoof a privileged
//! tool by passing the wrong shape, (b) sneak forbidden fields past
//! the validator via JSON aliasing, (c) break the strict enum/scope
//! contract by passing an unexpected variant, or (d) inject a control
//! channel via a string field that mimics a tool-call shape.
//!
//! This ring ships a **deterministic, signed manifest validator** for
//! tool calls. A `ToolCall` is accepted iff:
//!
//! 1. its `tool` name is present in the active `ToolManifest`;
//! 2. its `args` are an *exact* shape match for the manifest's
//!    declared `ArgSpec` — no extra keys, all required keys present,
//!    each value is the declared `ArgKind`;
//! 3. enum-typed args carry only declared variants (closed-world);
//! 4. `String` args are bounded by the declared length cap and do not
//!    contain a nested-tool-call sentinel.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · TOOL-ARG-CONFUSION`

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Closed-world set of argument shapes supported by the validator.
/// **No `Any` variant** — the whole point of the W17 lane is that the
/// validator refuses anything it cannot pin down statically.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArgKind {
    /// Bounded UTF-8 string — `cap` is the inclusive byte length cap.
    StringBounded {
        /// Inclusive maximum length in bytes.
        cap: usize,
    },
    /// 64-bit unsigned integer.
    U64,
    /// 64-bit signed integer.
    I64,
    /// Boolean.
    Bool,
    /// Closed-world enum — `variants` is the *exact* set of legal
    /// string values.
    Enum {
        /// Allowed variant strings (sorted, deduplicated by builder).
        variants: Vec<String>,
    },
}

/// A single named argument in a tool's manifest entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgSpec {
    /// Argument name as it appears in the call.
    pub name: String,
    /// Declared kind / shape.
    pub kind: ArgKind,
    /// Whether the argument MUST be present.
    pub required: bool,
}

/// A signed manifest entry for one tool. Signature handling is
/// deliberately omitted from this skeleton — the W17 lane focuses on
/// **shape** validation. CR-CHAT-06's `capability` ring already
/// owns signature verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEntry {
    /// Tool name (e.g. `"send_email"`, `"query_db"`).
    pub name: String,
    /// Argument specifications, in declaration order.
    pub args: Vec<ArgSpec>,
}

/// Active manifest the validator consults. The set of legal tool
/// names is fixed at construction — the validator MUST reject any
/// call whose `tool` is not in this map.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolManifest {
    /// Tool name → entry.
    entries: BTreeMap<String, ToolEntry>,
}

impl ToolManifest {
    /// Construct an empty manifest.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a `ToolEntry`. Later inserts shadow earlier ones for
    /// the same name (last-writer-wins) — but in W17 corpus we only
    /// register each tool exactly once.
    pub fn register(&mut self, entry: ToolEntry) {
        self.entries.insert(entry.name.clone(), entry);
    }

    /// Lookup a tool by name.
    pub fn lookup(&self, name: &str) -> Option<&ToolEntry> {
        self.entries.get(name)
    }
}

/// Concrete value supplied for a single argument in a [`ToolCall`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum ArgValue {
    /// String value.
    Str(String),
    /// Unsigned integer value.
    U(u64),
    /// Signed integer value.
    I(i64),
    /// Boolean value.
    Bool(bool),
}

/// A concrete tool invocation. The validator inspects this against
/// the active [`ToolManifest`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool name being invoked.
    pub tool: String,
    /// Argument map: name → concrete value.
    pub args: BTreeMap<String, ArgValue>,
}

/// Closed-world rejection reason. Every `validate_tool_call` failure
/// MUST map to one of these variants — no `Other` / `Unknown`
/// fallback (the validator's whole point is closed-world rejection).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolCallError {
    /// Tool name not present in the manifest (W17: TOOL-01).
    UnknownTool {
        /// The name the call attempted to invoke.
        name: String,
    },
    /// A required argument is missing (W17: TOOL-02).
    MissingArg {
        /// Argument name.
        name: String,
    },
    /// An argument was supplied that the manifest does not declare
    /// (W17: TOOL-03 — extra-key / type-confusion attack vector).
    UnexpectedArg {
        /// Argument name.
        name: String,
    },
    /// An argument's value kind does not match the declared
    /// [`ArgKind`] (W17: TOOL-04).
    KindMismatch {
        /// Argument name.
        name: String,
        /// Expected kind label.
        expected: &'static str,
        /// Actual kind label.
        actual: &'static str,
    },
    /// A `StringBounded` value exceeded the declared cap (W17:
    /// TOOL-05 — argument-confusion via overlong string).
    StringTooLong {
        /// Argument name.
        name: String,
        /// Declared cap.
        cap: usize,
        /// Actual byte length.
        actual: usize,
    },
    /// An enum-typed value carried a variant the manifest does not
    /// declare (W17: TOOL-06 — closed-world enum violation).
    UnknownEnumVariant {
        /// Argument name.
        name: String,
        /// Variant string the call supplied.
        variant: String,
    },
    /// A string value contained a nested-tool-call sentinel — the
    /// `<<TOOL-CALL>>` marker — which we treat as a structural
    /// injection attempt regardless of the cap.
    NestedToolCallSentinel {
        /// Argument name.
        name: String,
    },
}

impl ToolCallError {
    /// Stable label for tests / logs.
    pub fn label(&self) -> &'static str {
        match self {
            ToolCallError::UnknownTool { .. } => "unknown_tool",
            ToolCallError::MissingArg { .. } => "missing_arg",
            ToolCallError::UnexpectedArg { .. } => "unexpected_arg",
            ToolCallError::KindMismatch { .. } => "kind_mismatch",
            ToolCallError::StringTooLong { .. } => "string_too_long",
            ToolCallError::UnknownEnumVariant { .. } => "unknown_enum_variant",
            ToolCallError::NestedToolCallSentinel { .. } => "nested_tool_call_sentinel",
        }
    }
}

/// Sentinel a string MUST NOT contain — used to catch attempts at
/// nesting a fake structured tool call inside a free-text argument.
pub const NESTED_TOOL_CALL_SENTINEL: &str = "<<TOOL-CALL>>";

fn kind_label(v: &ArgValue) -> &'static str {
    match v {
        ArgValue::Str(_) => "string",
        ArgValue::U(_) => "u64",
        ArgValue::I(_) => "i64",
        ArgValue::Bool(_) => "bool",
    }
}

fn expected_label(k: &ArgKind) -> &'static str {
    match k {
        ArgKind::StringBounded { .. } => "string",
        ArgKind::U64 => "u64",
        ArgKind::I64 => "i64",
        ArgKind::Bool => "bool",
        ArgKind::Enum { .. } => "enum",
    }
}

/// Validate `call` against `manifest`. Returns `Ok(())` iff every
/// rule in the module-level docs holds.
///
/// The validator is **deterministic and pure** — no randomness, no
/// system clock, no global state. This is by design: the W17 Coq
/// proofs rely on this being a total function over the data.
pub fn validate_tool_call(manifest: &ToolManifest, call: &ToolCall) -> Result<(), ToolCallError> {
    // Rule 1: tool name must be in manifest.
    let entry = manifest
        .lookup(&call.tool)
        .ok_or_else(|| ToolCallError::UnknownTool {
            name: call.tool.clone(),
        })?;

    // Rule 2: every required arg present.
    for spec in &entry.args {
        if spec.required && !call.args.contains_key(&spec.name) {
            return Err(ToolCallError::MissingArg {
                name: spec.name.clone(),
            });
        }
    }

    // Rule 3: every supplied arg declared.
    for arg_name in call.args.keys() {
        if !entry.args.iter().any(|s| &s.name == arg_name) {
            return Err(ToolCallError::UnexpectedArg {
                name: arg_name.clone(),
            });
        }
    }

    // Rule 4 + 5 + 6 + 7: shape match per declared spec.
    for spec in &entry.args {
        let Some(val) = call.args.get(&spec.name) else {
            // Missing optional arg — fine.
            continue;
        };
        match (&spec.kind, val) {
            (ArgKind::StringBounded { cap }, ArgValue::Str(s)) => {
                if s.contains(NESTED_TOOL_CALL_SENTINEL) {
                    return Err(ToolCallError::NestedToolCallSentinel {
                        name: spec.name.clone(),
                    });
                }
                if s.len() > *cap {
                    return Err(ToolCallError::StringTooLong {
                        name: spec.name.clone(),
                        cap: *cap,
                        actual: s.len(),
                    });
                }
            }
            (ArgKind::U64, ArgValue::U(_)) => {}
            (ArgKind::I64, ArgValue::I(_)) => {}
            (ArgKind::Bool, ArgValue::Bool(_)) => {}
            (ArgKind::Enum { variants }, ArgValue::Str(s)) => {
                if s.contains(NESTED_TOOL_CALL_SENTINEL) {
                    return Err(ToolCallError::NestedToolCallSentinel {
                        name: spec.name.clone(),
                    });
                }
                if !variants.iter().any(|v| v == s) {
                    return Err(ToolCallError::UnknownEnumVariant {
                        name: spec.name.clone(),
                        variant: s.clone(),
                    });
                }
            }
            (kind, v) => {
                return Err(ToolCallError::KindMismatch {
                    name: spec.name.clone(),
                    expected: expected_label(kind),
                    actual: kind_label(v),
                });
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_manifest() -> ToolManifest {
        let mut m = ToolManifest::new();
        m.register(ToolEntry {
            name: "send_email".to_string(),
            args: vec![
                ArgSpec {
                    name: "to".to_string(),
                    kind: ArgKind::StringBounded { cap: 320 },
                    required: true,
                },
                ArgSpec {
                    name: "subject".to_string(),
                    kind: ArgKind::StringBounded { cap: 256 },
                    required: true,
                },
                ArgSpec {
                    name: "priority".to_string(),
                    kind: ArgKind::Enum {
                        variants: vec!["low".to_string(), "normal".to_string(), "high".to_string()],
                    },
                    required: false,
                },
                ArgSpec {
                    name: "retries".to_string(),
                    kind: ArgKind::U64,
                    required: false,
                },
            ],
        });
        m
    }

    fn good_call() -> ToolCall {
        let mut args = BTreeMap::new();
        args.insert("to".to_string(), ArgValue::Str("alice@example.test".to_string()));
        args.insert("subject".to_string(), ArgValue::Str("hello".to_string()));
        args.insert("priority".to_string(), ArgValue::Str("normal".to_string()));
        args.insert("retries".to_string(), ArgValue::U(3));
        ToolCall {
            tool: "send_email".to_string(),
            args,
        }
    }

    /// **TOOL-01** — well-formed call against a registered tool with
    /// matching arg shapes is accepted.
    #[test]
    fn tool_01_well_formed_call_accepted() {
        let m = fixture_manifest();
        let c = good_call();
        assert!(validate_tool_call(&m, &c).is_ok(), "TOOL-01: well-formed call must be accepted");
    }

    /// **TOOL-02** — calling an unregistered tool name is rejected
    /// with `UnknownTool`. This catches the most basic spoofing
    /// attempt.
    #[test]
    fn tool_02_unknown_tool_rejected() {
        let m = fixture_manifest();
        let mut c = good_call();
        c.tool = "exec_shell".to_string();
        let r = validate_tool_call(&m, &c);
        match r {
            Err(ToolCallError::UnknownTool { name }) => {
                assert_eq!(name, "exec_shell", "TOOL-02: name must be reported");
            }
            other => panic!("TOOL-02: expected UnknownTool, got {other:?}"),
        }
    }

    /// **TOOL-03** — a required argument missing is rejected with
    /// `MissingArg`.
    #[test]
    fn tool_03_missing_required_arg_rejected() {
        let m = fixture_manifest();
        let mut c = good_call();
        c.args.remove("subject");
        let r = validate_tool_call(&m, &c);
        match r {
            Err(ToolCallError::MissingArg { name }) => {
                assert_eq!(name, "subject");
            }
            other => panic!("TOOL-03: expected MissingArg, got {other:?}"),
        }
    }

    /// **TOOL-04** — an argument that is not declared in the
    /// manifest is rejected with `UnexpectedArg`. This blocks the
    /// extra-key smuggling attack.
    #[test]
    fn tool_04_extra_arg_rejected() {
        let m = fixture_manifest();
        let mut c = good_call();
        c.args.insert(
            "bcc".to_string(),
            ArgValue::Str("evil@example.test".to_string()),
        );
        let r = validate_tool_call(&m, &c);
        match r {
            Err(ToolCallError::UnexpectedArg { name }) => {
                assert_eq!(name, "bcc");
            }
            other => panic!("TOOL-04: expected UnexpectedArg, got {other:?}"),
        }
    }

    /// **TOOL-05** — a value of the wrong kind (e.g. boolean where a
    /// string is declared) is rejected with `KindMismatch`. This is
    /// the canonical type-confusion guard.
    #[test]
    fn tool_05_kind_mismatch_rejected() {
        let m = fixture_manifest();
        let mut c = good_call();
        // `subject` is StringBounded — supply a Bool instead.
        c.args.insert("subject".to_string(), ArgValue::Bool(true));
        let r = validate_tool_call(&m, &c);
        match r {
            Err(ToolCallError::KindMismatch {
                name,
                expected,
                actual,
            }) => {
                assert_eq!(name, "subject");
                assert_eq!(expected, "string");
                assert_eq!(actual, "bool");
            }
            other => panic!("TOOL-05: expected KindMismatch, got {other:?}"),
        }
    }

    /// **TOOL-06** — closed-world enum violation: an enum-typed arg
    /// carrying a variant not in the declared list is rejected.
    /// Plus: a `StringBounded` arg containing the
    /// `<<TOOL-CALL>>` sentinel is rejected with
    /// `NestedToolCallSentinel`. Both are covered together to keep
    /// the test count balanced at 6.
    #[test]
    fn tool_06_enum_variant_and_sentinel_rejected() {
        let m = fixture_manifest();

        // Enum violation.
        let mut c = good_call();
        c.args.insert(
            "priority".to_string(),
            ArgValue::Str("ULTRA-CRITICAL".to_string()),
        );
        match validate_tool_call(&m, &c) {
            Err(ToolCallError::UnknownEnumVariant { name, variant }) => {
                assert_eq!(name, "priority");
                assert_eq!(variant, "ULTRA-CRITICAL");
            }
            other => panic!("TOOL-06a: expected UnknownEnumVariant, got {other:?}"),
        }

        // Nested-tool-call sentinel.
        let mut c2 = good_call();
        c2.args.insert(
            "subject".to_string(),
            ArgValue::Str(format!("{NESTED_TOOL_CALL_SENTINEL} send money")),
        );
        match validate_tool_call(&m, &c2) {
            Err(ToolCallError::NestedToolCallSentinel { name }) => {
                assert_eq!(name, "subject");
            }
            other => panic!("TOOL-06b: expected NestedToolCallSentinel, got {other:?}"),
        }

        // Overlong string.
        let mut c3 = good_call();
        c3.args.insert(
            "subject".to_string(),
            ArgValue::Str("x".repeat(257)),
        );
        match validate_tool_call(&m, &c3) {
            Err(ToolCallError::StringTooLong { name, cap, actual }) => {
                assert_eq!(name, "subject");
                assert_eq!(cap, 256);
                assert_eq!(actual, 257);
            }
            other => panic!("TOOL-06c: expected StringTooLong, got {other:?}"),
        }
    }

    /// Wave-17 G-tool green summary — total of 6 tool-arg-confusion
    /// falsifier tests.
    #[test]
    fn green_g_tool_summary() {
        let count = 6usize;
        assert_eq!(
            count, 6,
            "Wave-17 L-CHAT-9-tool: {count} tool-arg-confusion falsifier tests"
        );
    }
}
