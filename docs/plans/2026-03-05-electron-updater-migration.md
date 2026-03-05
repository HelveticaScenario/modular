# electron-updater Migration Plan

**Goal:** Replace Electron's built-in `autoUpdater` (Squirrel-only) with `electron-updater` from the
electron-builder ecosystem to enable true auto-update on Linux (`.deb`, `.rpm`) in addition to the
existing macOS and Windows flows.

---

## Background and constraints

### Current state

| Platform | Build artifact                        | Update mechanism                                      |
| -------- | ------------------------------------- | ----------------------------------------------------- |
| macOS    | `Operator-darwin-arm64-X.Y.Z.zip`     | Squirrel.Mac via `update.electronjs.org`              |
| Windows  | `Operator-X.Y.Z.Setup.exe` (Squirrel) | Squirrel.Windows via `update.electronjs.org`          |
| Linux    | `.deb`, `.rpm`                        | **Browser fallback only** — opens GitHub release page |

### Build toolchain

The project uses **Electron Forge** (`@electron-forge/cli`, `@electron-forge/maker-*`,
`@electron-forge/publisher-github`), not electron-builder.

`electron-updater` is a standalone npm package and does **not** require switching to
electron-builder for packaging. However, it reads release metadata from `latest.yml` /
`latest-mac.yml` / `latest-linux.yml` files that must be present alongside the release
artifacts on GitHub. Forge's `publisher-github` does **not** generate these files; they must be
produced separately.

---

## Changes required

### 1. Add `electron-updater` dependency

```bash
yarn add electron-updater
```

`electron-updater` ships its own TypeScript declarations — no separate `@types` package needed.

### 2. Generate release metadata YAML files in CI

`electron-updater` locates updates by fetching `latest-mac.yml` / `latest-linux.yml` /
`latest.yml` from the GitHub release. Each file contains:

```yaml
version: 0.0.27
files:
    - url: Operator-darwin-arm64-0.0.27.zip
      sha512: <base64-sha512>
      size: 98765432
path: Operator-darwin-arm64-0.0.27.zip
sha512: <base64-sha512>
releaseDate: '2026-03-05T00:00:00.000Z'
```

**What to add:**

- Write `scripts/generate-update-metadata.mjs` — a small Node script that takes a list of
  artifact paths and produces the appropriate YAML file(s) using `crypto.createHash('sha512')`.
- In `.github/workflows/release.yml`, add steps **after** `yarn forge-publish` in each matrix
  job to:
    1. Download the just-published release asset (`gh release download vX.Y.Z --pattern "*.zip"`
       on macOS, `*.AppImage` / `*.deb` / `*.rpm` on Linux, `*.exe` on Windows).
    2. Run `node scripts/generate-update-metadata.mjs` to produce the YAML.
    3. Upload the YAML: `gh release upload vX.Y.Z latest-mac.yml` (or the appropriate name).

The GitHub token already present in CI (`GITHUB_TOKEN`) has permission to upload release assets.

> **Note on timing:** `forge-publish` creates the release draft and uploads assets atomically per
> platform in each matrix job. The YAML upload step in the same job runs after that job's assets
> exist, so there is no race between jobs.

### 3. Add AppImage to Linux builds

`electron-updater` supports Linux in three modes:

| Format       | Update UX                                                                 |
| ------------ | ------------------------------------------------------------------------- |
| **AppImage** | In-place, no privilege required — best experience                         |
| **DEB**      | Downloads `.deb` then runs `pkexec dpkg -i …` — requires PolicyKit prompt |
| **RPM**      | Downloads `.rpm` then runs `pkexec rpm -U …` — requires PolicyKit prompt  |

AppImage is the only format that updates silently without an OS-level privilege dialog. The
project currently builds `.deb` and `.rpm` but not AppImage.

**What to add:**

- Install `@reforged/maker-appimage` (the only maintained community AppImage maker for Forge):

    ```bash
    yarn add -D @reforged/maker-appimage
    ```

- Add to `forge.config.ts` `makers` array:

    ```typescript
    import { MakerAppImage } from '@reforged/maker-appimage';

    // inside makers: [...]
    new MakerAppImage({
        options: {
            bin: 'Operator',
            categories: ['AudioVideo', 'Music'],
        },
    }),
    ```

- Update `latest-linux.yml` metadata generation to reference the AppImage artifact. The `.deb`
  and `.rpm` artifacts remain and continue to be published (for users who install via package
  manager); `electron-updater` on Linux will use the AppImage path for auto-update.

> **Fallback for DEB/RPM users:** If the user installed via `.deb` or `.rpm`, electron-updater
> will still attempt the DEB/RPM update path (downloading the matching artifact and invoking
> `pkexec`). This may succeed on systems with PolicyKit configured, or fail silently on minimal
> desktop environments. The banner should remain in the `available` state with a "View Release"
> link as a fallback if the install fails — see Task 5.

### 4. Windows: Squirrel → NSIS (optional, breaking)

`electron-updater` for Windows requires **NSIS** installers, not Squirrel. The current project
uses `MakerSquirrel`.

There is no official `@electron-forge/maker-nsis`. Options:

- **Option A — Community maker:** Use `@reforged/maker-nsis` or `electron-wix-msi`.
- **Option B — Defer:** Keep Windows on the current Squirrel + `update.electronjs.org` flow
  (`autoUpdater` from `electron`), and only use `electron-updater` on macOS and Linux.

**Recommendation for this plan: defer Windows.** Windows auto-update already works reliably.
Squirrel.Windows → NSIS is a one-time breaking install step for all existing Windows users
(they would need to manually run the new installer; auto-update would not carry them over). This
migration is a separate, opt-in change that should be planned independently.

If/when Windows is migrated, the NSIS installer replaces the Squirrel installer in CI and the
`latest.yml` metadata file covers the `.exe` artifact.

### 5. Update `src/main/main.ts`

**Remove:**

- `import { autoUpdater } from 'electron'`
- `const GITHUB_REPO`, `UPDATE_FEED_URL`, `GITHUB_API_URL` constants
- `checkForUpdateAvailability()` function (entire GitHub API check)
- `latestReleaseUrl` module variable
- `initAutoUpdater()` function and its `setFeedURL` call
- `supportsAutoUpdater` constant and all guards on it
- `UPDATE_FEED_URL`-related `feedPlatform` / `darwin-${process.arch}` workaround (no longer
  needed — electron-updater resolves the correct artifact from `latest-mac.yml` directly)

**Add:**

- `import { autoUpdater } from 'electron-updater'`
- Early in the Update Service block:

    ```typescript
    autoUpdater.autoDownload = false;
    autoUpdater.autoInstallOnAppQuit = false;
    autoUpdater.allowPrerelease = false;
    ```

- Wire events:

    ```typescript
    autoUpdater.on('update-available', (info) => {
        const config = loadConfig();
        if (config.skippedUpdateVersion === info.version) return;
        const releaseUrl = `https://github.com/${GITHUB_REPO}/releases/tag/v${info.version}`;
        sendUpdateEvent(IPC_CHANNELS.UPDATE_AVAILABLE, {
            version: info.version,
            releaseUrl,
        } satisfies UpdateAvailableInfo);
    });

    autoUpdater.on('update-not-available', () => {
        // Normal; no banner needed. Log at debug level.
    });

    autoUpdater.on('download-progress', (progress) => {
        sendUpdateEvent(
            IPC_CHANNELS.UPDATE_DOWNLOAD_PROGRESS,
            progress.percent,
        );
    });

    autoUpdater.on('update-downloaded', () => {
        sendUpdateEvent(IPC_CHANNELS.UPDATE_DOWNLOADED);
    });

    autoUpdater.on('error', (err: Error) => {
        console.error('[update] error:', err.message);
        sendUpdateEvent(IPC_CHANNELS.UPDATE_ERROR, err.message);
    });
    ```

- Update the `UPDATE_CHECK` IPC handler:

    ```typescript
    ipcMain.handle(IPC_CHANNELS.UPDATE_CHECK, () => {
        if (app.isPackaged) autoUpdater.checkForUpdates();
    });
    ```

- Update the `UPDATE_DOWNLOAD` handler — no platform guard needed:

    ```typescript
    ipcMain.handle(IPC_CHANNELS.UPDATE_DOWNLOAD, () => {
        autoUpdater.downloadUpdate();
    });
    ```

- Remove the `UPDATE_INSTALL` Linux guard; `quitAndInstall()` is now universal.

- Update the `app.on('ready')` block to call `autoUpdater.checkForUpdates()` directly (no
  `initAutoUpdater()` or `checkForUpdateAvailability()` wrappers).

**electron-updater configuration:** For the GitHub provider, `electron-updater` reads the
`publish` configuration from `package.json` (under a `build.publish` key, same as electron-
builder). Add to `package.json`:

```json
{
    "build": {
        "publish": {
            "provider": "github",
            "owner": "HelveticaScenario",
            "repo": "operator"
        }
    }
}
```

### 6. Add `UPDATE_DOWNLOAD_PROGRESS` IPC channel (optional, enhancement)

electron-updater's `download-progress` event provides `{ percent, bytesPerSecond, transferred,
total }`. The current banner shows only "Downloading…" with no progress indicator. A new
`UPDATE_DOWNLOAD_PROGRESS` channel carrying `percent` can update the banner message (e.g.
"Downloading 42%…") with minimal renderer changes.

---

## Feature degradation and risks

| Area                                        | Risk / Change                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| ------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Windows auto-update**                     | Deferred to a future migration. Windows users continue on current Squirrel flow. If the Windows Squirrel and Linux electron-updater coexist in main.ts, two `autoUpdater` objects must be managed carefully to avoid import conflicts — likely requires a conditional `require()` or platform-specific module. See note below.                                                                                                                        |
| **macOS feed URL workaround**               | The `darwin-${process.arch}` workaround can be removed. electron-updater resolves architecture from the system and picks the correct asset from `latest-mac.yml`.                                                                                                                                                                                                                                                                                     |
| **`update-not-available` treated as error** | Current code fires `UPDATE_ERROR` on `update-not-available`. This must be fixed — it is a normal non-event in electron-updater and should be handled silently.                                                                                                                                                                                                                                                                                        |
| **`app.isPackaged` guard**                  | electron-updater's `checkForUpdates()` is a no-op in dev unless `forceDevUpdateConfig` is set. The existing `app.isPackaged` guard can stay as an extra safety, but is less critical.                                                                                                                                                                                                                                                                 |
| **`latest-mac.yml` first-boot**             | Existing users on the Squirrel `update.electronjs.org` flow will not receive the metadata-only update check until they upgrade to the version that ships electron-updater. For one release cycle, the old feed URL must remain live for `update.electronjs.org` to serve those users. No action needed — the service will simply 404 for new releases that don't publish to Squirrel format, which is the current behavior for "no update available". |
| **Linux DEB/RPM users**                     | Users who installed via `.deb` or `.rpm` will get the auto-update attempt but may hit a PolicyKit prompt or silent failure if permissions aren't configured. AppImage users get a seamless experience. The banner should offer a "View Release" link as a safe fallback.                                                                                                                                                                              |
| **Release metadata generation**             | The CI YAML generation step runs after `forge-publish`. If the publish step fails partway (e.g. partial asset upload), the YAML generation must be skipped or the release may have incomplete metadata. Guard the metadata step with `if: success()` in CI.                                                                                                                                                                                           |

### Note on coexisting Squirrel (Windows) + electron-updater (macOS/Linux)

If Windows migration is deferred, `src/main/main.ts` would need to import from `electron` on
Windows and from `electron-updater` on macOS/Linux. The cleanest way:

```typescript
const autoUpdater =
    process.platform === 'win32'
        ? require('electron').autoUpdater
        : require('electron-updater').autoUpdater;
```

This avoids Electron's `autoUpdater` attempting to handle Linux (where it would throw) while
keeping Windows on the current flow. The `setFeedURL` call for Windows stays inside the
`win32` branch.

---

## Task summary

| #   | Task                                                                       | Files touched                                                                                        | Risk                                 |
| --- | -------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- | ------------------------------------ |
| 1   | `yarn add electron-updater` + add `build.publish` to `package.json`        | `package.json`                                                                                       | Low                                  |
| 2   | Write `scripts/generate-update-metadata.mjs`                               | `scripts/` (new file)                                                                                | Medium                               |
| 3   | Update `.github/workflows/release.yml` to generate + upload `latest-*.yml` | `.github/workflows/release.yml`                                                                      | Medium — CI is the integration point |
| 4   | Add `MakerAppImage` via `@reforged/maker-appimage`                         | `forge.config.ts`, `package.json`                                                                    | Low — additive only                  |
| 5   | Refactor `src/main/main.ts` update service                                 | `src/main/main.ts`                                                                                   | Medium — core update logic           |
| 6   | Add `UPDATE_DOWNLOAD_PROGRESS` channel + update banner (optional)          | `src/shared/ipcTypes.ts`, `src/renderer/components/UpdateNotification.tsx`, `src/preload/preload.ts` | Low                                  |

Tasks 1 → 2 → 3 must run in order (dependency). Tasks 4, 5, 6 are independent of each other
but 5 depends on 1. Test locally by running `yarn start` and triggering `UPDATE_CHECK` via the
menu; in the packaged build, the full flow is exercisable by releasing a version bump.
