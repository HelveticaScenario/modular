# Task Completion Checklist
- Run relevant tests: backend (`cd modular_server; cargo test`) and/or core (`cd modular_core; cargo test`); frontend if touched (`cd modular_web; pnpm test`).
- Regenerate TS types if Rust types changed: `cd modular_web; pnpm run codegen` (or `cargo test export_types -- --ignored`).
- Format/lint as needed: `cargo fmt`, `cargo clippy`, `pnpm lint` if applicable.
- Verify dev servers still start: backend `cargo run` (port 3000), frontend `pnpm dev`.
- Summarize changes and follow repo conventions; avoid altering unrelated pending changes.
