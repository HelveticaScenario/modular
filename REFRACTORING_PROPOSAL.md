# Refactoring Proposal for src/

## Executive Summary

After a thorough review of the src/ directory and related code, I identified architectural issues, anti-patterns, DRY violations, and opportunities for improvement. This proposal covers structural changes and specific code-level improvements with a focus on maintainability, reliability, and clarity.

## 1. Critical Architectural Issues

### 1.1 God Component: App.tsx

Problem: src/App.tsx is monolithic with 13+ useEffect hooks and mixed responsibilities.

Issues:
- Manages workspace, filesystem, audio state, editor buffers, scope views, and error handling in one component.
- Duplicate Monaco cancellation error suppression logic.
- Multiple refs used to avoid stale closures (handleSubmitRef, handleStopRef, etc.).
- Inline async handlers in render tree create new functions each render.

Recommendation:
- Extract responsibilities into focused hooks and context providers.

Proposed structure:
- src/app/hooks/useWorkspace.ts
- src/app/hooks/useAudioEngine.ts
- src/app/hooks/usePatchExecution.ts
- src/app/hooks/useMenuShortcuts.ts
- src/app/context/WorkspaceContext.tsx
- src/app/context/AudioContext.tsx

### 1.2 Duplicate IPC Type Definitions

Problem:
- preload.ts defines ElectronAPI interface that duplicates ipcTypes.ts and IPCHandlers definitions.
- AppConfig is defined in ipcTypes.ts and main.ts.

Recommendation:
- Define IPC types once in ipcTypes.ts and derive ElectronAPI from them.
- Use zod inference for AppConfig and export the type.

### 1.3 Config Schema Defined Multiple Times

Problem:
- main.ts has Zod schema.
- ipcTypes.ts has AppConfig interface.
- configSchema.ts has JSON schema for Monaco.

Recommendation:
- Define the schema once using Zod.
- Generate JSON schema from Zod for Monaco.
- Export the inferred TypeScript type from the same source.

## 2. DRY Violations and Duplicate Patterns

### 2.1 Monaco Type Alias Duplication

COMPLETED

### 2.2 Error Handling Pattern Repetition in App.tsx

COMPLETED

### 2.3 Menu Event Subscription Boilerplate in preload.ts

COMPLETED

### 2.4 isPlainObject Duplication

Duplicated in:
- src/patchSimilarityRemap.ts
- src/dsl/paramsSchema.ts

Recommendation:
- Move to src/utils/typeGuards.ts

### 2.5 File Path Normalization Logic

Pattern duplicated in:
- src/app/buffers.ts
- src/app/hooks/useEditorBuffers.ts
- src/main.ts

Recommendation:
- Consolidate in src/utils/pathUtils.ts

## 3. Anti-Patterns Requiring Correction

### 3.1 Refs as Escape Hatches

Multiple refs in App.tsx are used to avoid stale closures, indicating problems with dependency management.

Recommendation:
- Use stable callbacks with proper dependencies.
- Consider useEvent pattern or split into smaller hooks.

### 3.2 Magic Strings

Examples:
- 'modular_unsaved_buffers'
- 'root', 'root_clock'
- widget IDs and seq IDs

Recommendation:
- Centralize in src/constants.ts

### 3.3 Inconsistent IPC Error Handling

Mix of sync and async operations, inconsistent error shapes.

Recommendation:
- Standardize all file operations as async.
- Return consistent result types for IPC handlers.

### 3.4 @ts-ignore Usage

Examples:
- main.ts registerIPCHandler
- preload.ts invokeIPC
- factories.ts positionalArgs

Recommendation:
- Fix type signatures rather than suppress errors.

### 3.5 Console Logging in Production

Console logs found in executor, GraphBuilder, App.tsx, etc.

Recommendation:
- Replace with debug logger gated by environment flag.

## 4. Legacy Code and Technical Debt

### 4.1 Global window debug hooks

COMPLETED

### 4.2 Unused or Vestigial Code

Potential unused elements:
- SCRATCH_FILE in FileExplorer.tsx
- legacy functions in GraphBuilder.ts

Recommendation:
- Remove or document if still required.

### 4.3 Inline Type Assertions

Example in useEditorBuffers.ts with as any.

Recommendation:
- Improve union narrowing and remove unsafe assertions.

## 5. File Organization Improvements

Current structure mixes domain logic with UI and editor integration.

Proposed reorganization:

src/
- core/
  - dsl/
  - synthesizer/
  - patch/
- editor/
  - monaco/
  - buffers/
- electron/
  - ipc/
  - menu/
  - windows/
- shared/
  - utils/
  - constants.ts
- ui/
  - components/
  - themes/

## 6. File-Specific Recommendations

### main.ts (1003 lines)

Problem:
- Handles IPC, filesystem, config, window, menu in one file.

Recommendation:
- Split into:
  - electron/main.ts
  - electron/ipc/handlers.ts
  - electron/ipc/filesystem.ts
  - electron/menu.ts
  - electron/config.ts
  - electron/windows/main.ts
  - electron/windows/help.ts

### useEditorBuffers.ts (502 lines)

Problem:
- Large hook with 17+ returned functions and state values.

Recommendation:
- Split into:
  - useBufferState.ts
  - useFileOperations.ts
  - useBufferNavigation.ts

### typescriptLibGen.ts (758 lines)

Problem:
- Large static library string embedded in code.

Recommendation:
- Move base lib to .d.ts and generate only module-specific types.

## 7. Type Safety Improvements

### IPC Type Safety

Problem:
- Computed property keys in IPCHandlers lose type information.

Recommendation:
- Use mapped types with explicit handler signatures.

### Discriminated Union Usage

Problem:
- EditorBuffer not consistently narrowed.

Recommendation:
- Use explicit kind checks before accessing filePath.

## 8. Performance Considerations

### App.tsx Re-renders

Inline async handlers in JSX trigger re-renders.

Recommendation:
- Use useCallback and move handlers out of JSX.

### Scope Polling

requestAnimationFrame loop calls IPC on each frame.

Recommendation:
- Throttle or buffer results, or reduce polling frequency when idle.

## 9. Testing Gaps

Current tests are limited to patchSimilarityRemap.

Recommendation:
- Add tests for DSL execution, buffer management, IPC handlers, and UI components.

## 10. Suggested Implementation Order

1. High Priority
- Extract duplicate utilities.
- Centralize constants.
- Add debug logging wrapper.

2. Medium Priority
- Split main.ts into modules.
- Refactor App.tsx into hooks.
- Remove @ts-ignore usage.

3. Lower Priority
- Reorganize folder structure.
- Improve IPC typing.
- Expand test coverage.
