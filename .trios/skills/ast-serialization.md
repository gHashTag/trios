# Skill: AST Serialization (AST-to-Source)

## When to use

When a tool modifies an AST (e.g., sorts declarations, applies refactors) and needs to output the modified AST as valid source code instead of discarding changes.

## Why

- Sorting or transforming an AST is useless if the output prints the original source.
- A robust serializer enables code formatters, sorters, and auto-fixers.
- Round-trip parsing → serialization → re-parsing validates AST completeness.

## Steps

1. **Identify missing serialization:** Find tools that modify AST but print `source` string.
2. **Implement recursive `serialize_node`:**
   ```rust
   fn serialize_node(node: &Node, indent: usize) -> String {
       let pad = "  ".repeat(indent);
       match node.kind {
           NodeKind::FnDecl => {
               let mut out = format!("{}fn {}(...) {{\n", pad, node.name);
               for child in &node.children {
                   out.push_str(&serialize_node(child, indent + 1));
               }
               out.push_str(&format!("{}}}\n", pad));
               out
           }
           // ... other kinds
       }
   }
   ```
3. **Handle braces in Rust format strings:** Escape `{{` and `}}` in `format!()`:
   ```rust
   format!("{}fn {}() {{\n", pad, node.name) // WRONG — `{{` needed
   format!("{}fn {}() {{\n", pad, node.name) // RIGHT
   ```
4. **Support all major kinds:** Module, declarations, statements, expressions.
5. **Verify round-trip:** Parse → Serialize → Parse should produce equivalent AST.

## Common Pitfalls

- **Unescaped braces in `format!`:** Rust interprets `{}` as format placeholder. Literal braces must be `{{` and `}}`.
- **Missing indentation:** Recurse with `indent + 1` and use `"  ".repeat(indent)`.
- **Incomplete kind coverage:** Add a fallback `_ =>` that prints name/value for unknown kinds.

## Related

- [[ast-driven-codegen]] — Generating assembly/IR from AST
- [[l3-purity-audit]] — Ensuring serialized output is ASCII-only
