# Design: Add knip + oxlint

**Date:** 2026-04-01  
**Branch:** feature/linting

## Summary

Add two complementary static analysis tools to the project:

- **knip** — detects unused exports, files, and dependencies
- **oxlint** — fast Rust-based JS/TS linter covering correctness, style, suspicious patterns, TypeScript, and React rules

Both tools run as yarn scripts and block commits in the pre-commit hook.

---

## Tools

### knip

- Finds unused exports, files, and `package.json` dependencies
- Configured via `knip.json` at the repo root
- Entry points:
    - `src/main/main.ts`
    - `src/renderer/index.tsx`
    - `src/preload/preload.ts`
    - `vite.main.config.ts`, `vite.preload.config.ts`, `vite.renderer.config.ts`
    - `forge.config.ts`
    - `vitest.config.ts`
    - `playwright.config.ts`
- `crates/` and `scripts/` are excluded from TypeScript analysis

### oxlint

- Configured via `.oxlintrc.json` at the repo root
- All rule categories enabled: `correctness`, `suspicious`, `pedantic`, `style`, `restriction`, `nursery`, React plugin, TypeScript plugin
- Run with `oxlint src`

---

## Scripts (`package.json`)

```json
"lint":     "oxlint src",
"lint:fix": "oxlint src --fix",
"knip":     "knip"
```

---

## Pre-commit hook

`.husky/pre-commit` updated to run both tools before lint-staged:

```sh
yarn lint
yarn knip
npx lint-staged
```

Both `yarn lint` and `yarn knip` are hard-failing — a non-zero exit code blocks the commit.

---

## Decisions

| Decision           | Choice               | Reason                                |
| ------------------ | -------------------- | ------------------------------------- |
| Integration        | Pre-commit + scripts | Automatic enforcement + manual CI use |
| knip in pre-commit | Blocking             | Keep the branch clean                 |
| oxlint rule set    | All categories       | Strict from the start                 |
