# Compile t27c

Builds the t27c bootstrap compiler.

## Usage

```
/compile
```

## What It Does

1. Runs `cargo build --release -p t27c`
2. Shows summary:
   - Build time
   - Number of warnings
   - Number of errors
3. If errors occur, shows first 10 errors

## Output

```
Building t27c...
✅ Build succeeded
Warnings: 5
Errors: 0
Time: 2.3s
```

## Example with Errors

```
Building t27c...
❌ Build failed
Errors: 27

Top 10 errors:
1. error[E0761]: file for module `compiler` found at both...
2. error[E0412]: cannot find type `TaskCommands`...
3. ...
```

## Prerequisites

- Rust toolchain installed
- In `~/t27/bootstrap/` directory
