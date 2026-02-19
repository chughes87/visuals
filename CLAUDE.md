# CLAUDE.md — Notes for Claude

## Project Overview

This repository contains a Fractal Explorer implemented in two languages:
- **Clojure + Quil** — the original implementation (`src/`, `project.clj`)
- **Rust + wgpu** — the in-progress GPU rewrite (`rust/`)

The Rust rewrite follows the phased plan in `rust/RUST_GPU_PLAN.md`.

## Rust Workspace

The Rust code lives under `rust/` as a Cargo workspace with three crates:
- `crates/fractal-core` — pure Rust types, CPU logic, presets, modulators (no GPU deps)
- `crates/fractal-gpu` — wgpu pipelines, WGSL shaders, GPU buffers
- `crates/fractal-app` — winit event loop, main binary

```
cd rust/
cargo build -p fractal-app   # build the app
cargo run -p fractal-app     # run the app
cargo test                   # run all tests
```

## Before Pushing

**Always run the formatter and fix any issues before committing or pushing:**

```
cd rust/
cargo fmt          # auto-format all crates
cargo fmt --check  # verify no formatting differences remain
```

If `cargo fmt --check` reports diffs, run `cargo fmt` and re-stage the affected files before committing.

## Development Workflow

1. Implement the next phase from `rust/RUST_GPU_PLAN.md`
2. Run `cargo build -p fractal-app` — fix any compile errors
3. Run `cargo test` — all tests must pass
4. Run `cargo fmt` — format the code
5. Run `cargo fmt --check` — confirm clean
6. Commit and push to the feature branch
