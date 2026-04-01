# knip + oxlint Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add knip (dead-code/unused-dependency detector) and oxlint (fast JS/TS linter) to the project, wired into yarn scripts and the pre-commit hook.

**Architecture:** Install both tools as devDependencies, create their config files (`knip.json`, `.oxlintrc.json`), add `lint`, `lint:fix`, and `knip` yarn scripts, and update `.husky/pre-commit` to run them before lint-staged. No test files are needed — verification is "tool runs without crashing and exits 0 on the current codebase."

**Tech Stack:** knip, oxlint, Yarn 4, Husky

---

### Task 1: Install knip and oxlint

**Files:**

- Modify: `package.json` (devDependencies — Yarn handles this automatically)

**Step 1: Install both packages**

```bash
yarn add -D knip oxlint
```

**Step 2: Verify installation**

```bash
yarn knip --version
yarn oxlint --version
```

Expected: both print a version number with exit code 0.

**Step 3: Commit**

```bash
git add package.json yarn.lock
git commit -m "chore: install knip and oxlint"
```

---

### Task 2: Create knip config

**Files:**

- Create: `knip.json`

**Step 1: Create `knip.json`**

```json
{
    "$schema": "https://unpkg.com/knip@latest/schema.json",
    "entry": [
        "src/main/main.ts",
        "src/renderer/renderer.tsx",
        "src/preload/preload.ts",
        "vite.main.config.ts",
        "vite.preload.config.ts",
        "vite.renderer.config.ts",
        "forge.config.ts",
        "vitest.config.ts",
        "playwright.config.ts",
        "scripts/**/*.{ts,mjs}"
    ],
    "project": ["src/**/*.{ts,tsx}"],
    "ignore": ["crates/**", "e2e/**", "public/**"]
}
```

**Step 2: Run knip to verify the config is valid**

```bash
yarn knip --reporter compact
```

Expected: exits 0 (or lists findings but does not crash). If it crashes with "entry file not found", fix the path.

**Step 3: Commit**

```bash
git add knip.json
git commit -m "chore: add knip config"
```

---

### Task 3: Create oxlint config

**Files:**

- Create: `.oxlintrc.json`

**Step 1: Create `.oxlintrc.json`**

```json
{
    "$schema": "https://cdn.jsdelivr.net/npm/oxlint/configuration_schema.json",
    "plugins": ["react", "typescript"],
    "categories": {
        "correctness": "error",
        "suspicious": "error",
        "pedantic": "warn",
        "style": "warn",
        "restriction": "warn",
        "nursery": "warn"
    },
    "env": {
        "browser": true,
        "node": true,
        "es2022": true
    }
}
```

**Step 2: Run oxlint to verify the config is valid**

```bash
yarn oxlint src --config .oxlintrc.json
```

Expected: exits 0 or exits with lint warnings/errors but does not crash with "invalid config". If there are lint errors, note them — they will be addressed in Task 5.

**Step 3: Commit**

```bash
git add .oxlintrc.json
git commit -m "chore: add oxlint config"
```

---

### Task 4: Add yarn scripts

**Files:**

- Modify: `package.json`

**Step 1: Add the three scripts to the `"scripts"` section**

In `package.json`, add inside the `"scripts"` object:

```json
"lint": "oxlint src --config .oxlintrc.json",
"lint:fix": "oxlint src --fix --config .oxlintrc.json",
"knip": "knip"
```

**Step 2: Verify the scripts work**

```bash
yarn lint --version
yarn knip --version
```

Run the actual lint (expect it may report issues — that's fine at this stage):

```bash
yarn lint 2>&1 | head -20
yarn knip --reporter compact 2>&1 | head -20
```

**Step 3: Commit**

```bash
git add package.json
git commit -m "chore: add lint and knip yarn scripts"
```

---

### Task 5: Fix any oxlint errors (not warnings)

**Files:**

- Various `src/**/*.{ts,tsx}` files as needed

**Step 1: Run lint and capture full output**

```bash
yarn lint 2>&1
```

**Step 2: Fix any `error`-level findings**

For each error-level finding, fix the source code. Common issues:

- Unused variables → remove or prefix with `_`
- `no-unused-expressions` → remove the expression
- TypeScript-specific errors → follow oxlint's suggestion

Do NOT fix warnings yet — only errors (to get the hook to pass).

**Step 3: Re-run to confirm zero errors**

```bash
yarn lint 2>&1
```

Expected: exits 0.

**Step 4: Commit all fixes**

```bash
git add -u
git commit -m "fix: resolve oxlint errors"
```

---

### Task 6: Fix any knip errors

**Files:**

- Various `src/**/*.{ts,tsx}` files and `package.json` as needed

**Step 1: Run knip and capture full output**

```bash
yarn knip --reporter compact 2>&1
```

**Step 2: Triage findings**

knip reports:

- **Unused files** — if genuinely unused, delete them; if they are test fixtures or generated, add them to `knip.json` `"ignore"`.
- **Unused exports** — if genuinely unused, remove the `export` keyword; if they are part of a public API used outside TypeScript (e.g., N-API surface), add to `knip.json` `"ignoreBinaries"` or mark with `// @knip-ignore`.
- **Unlisted/unused dependencies** — remove from `package.json` if not needed, or add to `knip.json` `"ignoreDependencies"` if they are indirect peer deps.

**Step 3: Re-run to confirm zero findings**

```bash
yarn knip --reporter compact 2>&1
```

Expected: exits 0 with no output, or "No issues found."

**Step 4: Commit all fixes**

```bash
git add -u
git commit -m "fix: resolve knip findings"
```

---

### Task 7: Update pre-commit hook

**Files:**

- Modify: `.husky/pre-commit`

**Step 1: Replace the pre-commit file contents**

Replace the existing single-line content:

```sh
npx lint-staged
```

With:

```sh
yarn lint
yarn knip
npx lint-staged
```

**Step 2: Make a test commit to verify the hook runs**

Stage any trivially changed file (e.g., add a trailing newline to `README.md`), then attempt a commit:

```bash
git add README.md
git commit -m "test: verify pre-commit hook"
```

Expected: `yarn lint` runs, `yarn knip` runs, `npx lint-staged` runs (formats staged files), commit succeeds.

**Step 3: If the hook commit test worked, commit the hook change itself**

```bash
git add .husky/pre-commit
git commit -m "chore: add lint and knip to pre-commit hook"
```

(If Task 7 Step 2 was used as the commit, amend or just note the hook file is already committed.)

---

### Task 8: Verify everything end-to-end

**Step 1: Run all linting tools clean**

```bash
yarn lint
yarn knip
```

Both should exit 0.

**Step 2: Run existing tests to confirm nothing broke**

```bash
yarn test:unit
```

Expected: all pass.

**Step 3: Done**

The feature is complete. Both tools are installed, configured, wired to yarn scripts, and enforced in the pre-commit hook.
