# Skill: AST-Driven Code Generation

## When to use

When a compiler or transpiler emits hardcoded output regardless of input AST, or when you need to generate target code (assembly, IR) from a parsed AST.

## Why

- Hardcoded output defeats the purpose of parsing — the AST is ignored.
- AST-driven generation ensures different inputs produce different outputs.
- Deterministic mapping from AST nodes to target instructions enables regression testing.

## Steps

1. **Identify the hardcode:** Find functions that emit fixed strings/instructions regardless of AST content.
2. **Design a recursive emitter:**
   ```rust
   fn emit_node(emitter: &mut Emitter, node: &Node, ctx: &mut Context) {
       match node.kind {
           NodeKind::FnDecl => { /* emit function prologue */ }
           NodeKind::ConstDecl => { /* emit constant load */ }
           NodeKind::ExprBinary => {
               emit_node(emitter, &node.children[0], ctx);
               emit_node(emitter, &node.children[1], ctx);
               emit_op(emitter, &node.extra_op);
           }
           _ => { /* recurse or skip */ }
       }
   }
   ```
3. **Map AST fields to target operands:**
   - `node.name` → symbol table entries
   - `node.value` → immediate values
   - `node.extra_op` → operator opcodes
   - `node.children` → nested expressions
4. **Register allocation:** Use a rotating register counter or simple allocation scheme.
5. **Verify determinism:** Same AST → same output. Different AST → different output.

## Example

Before (hardcoded):
```rust
asm.emit_r(0x01, 1, 27, 0);
asm.emit_i(0x03, 2, 1, 42);
asm.emit_r(0x01, 3, 2, 1);
```

After (AST-driven):
```rust
fn emit_node(asm: &mut HirAssembler, node: &Node, reg: &mut u32) {
    match node.kind {
        NodeKind::FnDecl => {
            asm.define_symbol(&node.name, true);
            asm.emit_r(0x01, *reg, 0, 0);
            *reg = (*reg % 7) + 1;
            for child in &node.children { emit_node(asm, child, reg); }
        }
        NodeKind::ConstDecl => {
            let imm: u32 = node.value.parse().unwrap_or(0);
            asm.emit_i(0x03, *reg, 0, imm);
            *reg = (*reg % 7) + 1;
        }
        _ => {}
    }
}
```

## Related

- [[ast-serialization]] — Converting AST back to source code
- [[compiler-bug-triage]] — Identifying and fixing compiler output bugs
