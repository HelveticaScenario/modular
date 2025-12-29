# Suggested Commands
- Run backend server: `cd modular_server; cargo run` (default http://localhost:3000).
- Run frontend dev server: `cd modular_web; yarn install; yarn dev` (http://localhost:5173).
- Generate TS types from Rust: `cd modular_web; yarn run codegen` (runs cargo test export_types under the hood).
- Backend tests: `cd modular_server; cargo test` or `cd modular_core; cargo test` (run per crate).
- Build release binaries: `cargo build --release` (from repo root or crate dir).
- Common utilities (fish/macOS): `ls`, `cd <dir>`, `rg <pattern>`, `fd <name>`, `git status`, `git diff`, `yarn install`, `cargo fmt`, `cargo clippy`.
