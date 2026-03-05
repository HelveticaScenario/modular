# In-App Updates (Opt-In) Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the silent auto-download behavior of `update-electron-app` with an explicit opt-in update flow: version availability is checked via the GitHub API (no download), a dismissible in-app banner is shown, and the actual download only begins when the user clicks "Download & Install."

**Architecture:** A GitHub API check (no download) runs at startup and on demand. Update state is bridged from the main process to the renderer via IPC push events. The renderer shows a non-blocking banner. The actual Squirrel download (`autoUpdater.checkForUpdates()`) is only triggered by user consent. "Skip this version" is persisted in `config.json`. Linux users get a banner with a browser link instead of an in-app install.

**Tech Stack:** Electron `autoUpdater` (built-in, macOS + Windows only), native `fetch` (Node 22), React, existing IPC/preload patterns in this project.

---

### Task 1: Add update types and channels to `src/shared/ipcTypes.ts`

**Files:**

- Modify: `src/shared/ipcTypes.ts`

No test for this task — it's pure types.

**Step 1: Add `UpdateAvailableInfo` type and `skippedUpdateVersion` to `AppConfig`**

Add after the `MainLogEntry` interface (around line 122):

```typescript
export interface UpdateAvailableInfo {
    /** Semver string, e.g. "0.0.25" */
    version: string;
    /** URL to the GitHub release page */
    releaseUrl: string;
}
```

Add `skippedUpdateVersion?: string` to the `AppConfig` interface (around line 107):

```typescript
export interface AppConfig {
    // ... existing fields ...
    skippedUpdateVersion?: string;
}
```

**Step 2: Add update channels to `IPC_CHANNELS`**

At the end of the `IPC_CHANNELS` object, before `} as const`, add:

```typescript
// Update operations
UPDATE_CHECK: 'modular:update:check',
UPDATE_DOWNLOAD: 'modular:update:download',
UPDATE_INSTALL: 'modular:update:install',
UPDATE_AVAILABLE: 'modular:update:available',
UPDATE_DOWNLOADING: 'modular:update:downloading',
UPDATE_DOWNLOADED: 'modular:update:downloaded',
UPDATE_ERROR: 'modular:update:error',
```

**Step 3: Add handler types to `IPCHandlers`**

At the end of the `IPCHandlers` interface, add:

```typescript
// Update operations (invokable)
[IPC_CHANNELS.UPDATE_CHECK]: () => void;
[IPC_CHANNELS.UPDATE_DOWNLOAD]: () => void;
[IPC_CHANNELS.UPDATE_INSTALL]: () => void;
// Update operations (push from main to renderer)
[IPC_CHANNELS.UPDATE_AVAILABLE]: (info: UpdateAvailableInfo) => void;
[IPC_CHANNELS.UPDATE_DOWNLOADING]: () => void;
[IPC_CHANNELS.UPDATE_DOWNLOADED]: () => void;
[IPC_CHANNELS.UPDATE_ERROR]: (message: string) => void;
```

**Step 4: Run typecheck**

```bash
yarn typecheck
```

Expected: passes (or only pre-existing errors).

---

### Task 2: Wire up update logic in `src/main/main.ts`

**Files:**

- Modify: `src/main/main.ts`

**Step 1: Replace `updateElectronApp` import with `autoUpdater`**

Remove:

```typescript
import { updateElectronApp } from 'update-electron-app';

if (app.isPackaged) {
    updateElectronApp({
        repo: 'HelveticaScenario/operator',
    });
}
```

Add to the existing electron import at the top:

```typescript
import { autoUpdater } from 'electron';
```

Also add `UpdateAvailableInfo` to the ipcTypes import:

```typescript
import {
    IPC_CHANNELS,
    IPCHandlers,
    FileTreeEntry,
    MENU_CHANNELS,
    ContextMenuOptions,
    DSLExecuteResult,
    MainLogLevel,
    MainLogEntry,
    UpdateAvailableInfo,
} from '../shared/ipcTypes';
```

**Step 2: Add `skippedUpdateVersion` to `AppConfigSchema`**

In `AppConfigSchema` (around line 292, after `audioConfig`), add:

```typescript
skippedUpdateVersion: z.string().optional(),
```

**Step 3: Add the update service block**

Add a new section after the log forwarding section (after line ~201), before the Squirrel startup handler:

```typescript
// ========================================================================
// Update Service
// ========================================================================

const GITHUB_REPO = 'HelveticaScenario/operator';
const UPDATE_FEED_URL = `https://update.electronjs.org/${GITHUB_REPO}`;
const GITHUB_API_URL = `https://api.github.com/repos/${GITHUB_REPO}/releases/latest`;

/** True on platforms where autoUpdater (Squirrel) is supported */
const supportsAutoUpdater =
    process.platform === 'darwin' || process.platform === 'win32';

function sendUpdateEvent(channel: string, ...args: unknown[]) {
    if (mainWindow && !mainWindow.isDestroyed()) {
        mainWindow.webContents.send(channel, ...args);
    }
}

/**
 * Fetches the latest release from GitHub API and pushes UPDATE_AVAILABLE
 * to the renderer if a newer version is available and not skipped.
 * Does NOT trigger any download.
 */
async function checkForUpdateAvailability(): Promise<void> {
    try {
        const response = await fetch(GITHUB_API_URL, {
            headers: { 'User-Agent': `${GITHUB_REPO}` },
        });
        if (!response.ok) {
            console.warn('[update] GitHub API returned', response.status);
            return;
        }
        const data = (await response.json()) as {
            tag_name: string;
            html_url: string;
        };
        const latestVersion = data.tag_name.replace(/^v/, '');
        const currentVersion = app.getVersion();
        if (latestVersion === currentVersion) return;

        // Compare semver — only notify if latest is strictly newer
        const [lMaj, lMin, lPat] = latestVersion.split('.').map(Number);
        const [cMaj, cMin, cPat] = currentVersion.split('.').map(Number);
        const isNewer =
            lMaj > cMaj ||
            (lMaj === cMaj && lMin > cMin) ||
            (lMaj === cMaj && lMin === cMin && lPat > cPat);

        if (!isNewer) return;

        // Check if this version was skipped
        const config = loadConfig();
        if (config.skippedUpdateVersion === latestVersion) return;

        const info: UpdateAvailableInfo = {
            version: latestVersion,
            releaseUrl: data.html_url,
        };
        sendUpdateEvent(IPC_CHANNELS.UPDATE_AVAILABLE, info);
    } catch (err) {
        console.warn('[update] Version check failed:', err);
    }
}

/**
 * Initialise the autoUpdater feed URL and event listeners.
 * Called once, after app.ready, only in packaged builds on supported platforms.
 */
function initAutoUpdater(): void {
    if (!supportsAutoUpdater) return;

    autoUpdater.setFeedURL({
        url: `${UPDATE_FEED_URL}/${process.platform}/${app.getVersion()}`,
    });

    autoUpdater.on('update-downloaded', () => {
        sendUpdateEvent(IPC_CHANNELS.UPDATE_DOWNLOADED);
    });

    autoUpdater.on('error', (err: Error) => {
        console.error('[update] autoUpdater error:', err.message);
        sendUpdateEvent(IPC_CHANNELS.UPDATE_ERROR, err.message);
    });
}
```

**Step 4: Add IPC handlers for update actions**

In the existing IPC handler section (find a logical place, e.g. near the config handlers), add:

```typescript
// Update operations
ipcMain.handle(IPC_CHANNELS.UPDATE_CHECK, async () => {
    await checkForUpdateAvailability();
});

ipcMain.handle(IPC_CHANNELS.UPDATE_DOWNLOAD, () => {
    if (!supportsAutoUpdater) return;
    sendUpdateEvent(IPC_CHANNELS.UPDATE_DOWNLOADING);
    autoUpdater.checkForUpdates();
});

ipcMain.handle(IPC_CHANNELS.UPDATE_INSTALL, () => {
    if (!supportsAutoUpdater) return;
    autoUpdater.quitAndInstall();
});
```

**Step 5: Call init functions in `app.on('ready')`**

In the `app.on('ready', ...)` handler (around line 1581), after `createMenu()`, add:

```typescript
if (app.isPackaged) {
    initAutoUpdater();
    // Delay first check slightly so the window is visible before a banner appears
    setTimeout(() => void checkForUpdateAvailability(), 5000);
}
```

**Step 6: Add "Check for Updates..." to the app menu**

In `createMenu()`:

For macOS (in the app submenu, after `{ role: 'about' as const }` and before the Settings separator), add:

```typescript
{ type: 'separator' as const },
{
    label: 'Check for Updates...',
    click: async () => {
        await checkForUpdateAvailability();
    },
},
```

For Windows/Linux (add a Help menu at the end of the template array, after the Window menu):

```typescript
// Help menu (non-macOS: includes Check for Updates)
...(!isMac
    ? [
          {
              label: 'Help',
              submenu: [
                  {
                      label: 'Check for Updates...',
                      click: async () => {
                          await checkForUpdateAvailability();
                      },
                  },
              ],
          },
      ]
    : []),
```

**Step 7: Run typecheck**

```bash
yarn typecheck
```

Expected: passes.

---

### Task 3: Expose update API in `src/preload/preload.ts`

**Files:**

- Modify: `src/preload/preload.ts`

**Step 1: Add imports**

Add `UpdateAvailableInfo` to the ipcTypes import at the top:

```typescript
import {
    IPC_CHANNELS,
    IPCHandlers,
    IPCRequest,
    IPCResponse,
    Promisify,
    MENU_CHANNELS,
    ContextMenuOptions,
    ContextMenuAction,
    AppConfig,
    DSLExecuteResult,
    MainLogEntry,
    UpdateAvailableInfo,
} from '../shared/ipcTypes';
```

**Step 2: Add `update` namespace to `ElectronAPI` interface**

After the `onMainLog` entry in the `ElectronAPI` interface, add:

```typescript
// Update operations
update: {
    check: () => Promise<void>;
    download: () => Promise<void>;
    install: () => Promise<void>;
    onAvailable: (callback: (info: UpdateAvailableInfo) => void) => () => void;
    onDownloading: (callback: () => void) => () => void;
    onDownloaded: (callback: () => void) => () => void;
    onError: (callback: (message: string) => void) => () => void;
};
```

**Step 3: Implement the `update` namespace in `electronAPI`**

After the `onMainLog` line in the `electronAPI` object, add:

```typescript
// Update operations
update: {
    check: () => ipcRenderer.invoke(IPC_CHANNELS.UPDATE_CHECK),
    download: () => ipcRenderer.invoke(IPC_CHANNELS.UPDATE_DOWNLOAD),
    install: () => ipcRenderer.invoke(IPC_CHANNELS.UPDATE_INSTALL),
    onAvailable: menuEventHandler<[UpdateAvailableInfo]>(
        IPC_CHANNELS.UPDATE_AVAILABLE,
    ),
    onDownloading: menuEventHandler(IPC_CHANNELS.UPDATE_DOWNLOADING),
    onDownloaded: menuEventHandler(IPC_CHANNELS.UPDATE_DOWNLOADED),
    onError: menuEventHandler<[string]>(IPC_CHANNELS.UPDATE_ERROR),
},
```

**Step 4: Run typecheck**

```bash
yarn typecheck
```

Expected: passes.

---

### Task 4: Create `UpdateNotification` component

**Files:**

- Create: `src/renderer/components/UpdateNotification.tsx`
- Create: `src/renderer/components/UpdateNotification.css`

This component receives its state as props and calls callbacks. The parent (`App.tsx`) will own the state.

**Step 1: Create the CSS file**

`src/renderer/components/UpdateNotification.css`:

```css
.update-notification {
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    z-index: 9999;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 16px;
    background: var(--color-bg-elevated, #1e1e2e);
    border-top: 1px solid var(--color-border, #313244);
    font-size: 13px;
    color: var(--color-text, #cdd6f4);
}

.update-notification__message {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.update-notification__actions {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
}

.update-notification__btn {
    padding: 3px 10px;
    border-radius: 4px;
    font-size: 12px;
    cursor: pointer;
    border: 1px solid transparent;
    background: transparent;
    color: inherit;
}

.update-notification__btn--primary {
    background: var(--color-accent, #89b4fa);
    color: var(--color-bg, #1e1e2e);
    border-color: var(--color-accent, #89b4fa);
}

.update-notification__btn--primary:hover {
    opacity: 0.85;
}

.update-notification__btn--secondary {
    color: var(--color-text-subtle, #a6adc8);
    border-color: var(--color-border, #313244);
}

.update-notification__btn--secondary:hover {
    background: var(--color-bg-hover, #313244);
}
```

**Step 2: Create the component**

`src/renderer/components/UpdateNotification.tsx`:

```typescript
import './UpdateNotification.css';

export type UpdateNotificationState =
    | { status: 'idle' }
    | { status: 'available'; version: string; releaseUrl: string }
    | { status: 'downloading'; version: string }
    | { status: 'ready' }
    | { status: 'error'; message: string };

interface Props {
    state: UpdateNotificationState;
    onDownload: () => void;
    onInstall: () => void;
    onSkip: () => void;
    onDismiss: () => void;
}

export function UpdateNotification({
    state,
    onDownload,
    onInstall,
    onSkip,
    onDismiss,
}: Props) {
    if (state.status === 'idle') return null;

    let message: string;
    let primaryAction: { label: string; onClick: () => void } | null = null;
    let secondaryActions: { label: string; onClick: () => void }[] = [];

    switch (state.status) {
        case 'available':
            message = `Version ${state.version} is available.`;
            primaryAction = { label: 'Download & Install', onClick: onDownload };
            secondaryActions = [
                { label: 'Skip This Version', onClick: onSkip },
                { label: 'Dismiss', onClick: onDismiss },
            ];
            break;
        case 'downloading':
            message = `Downloading ${state.version}…`;
            break;
        case 'ready':
            message = 'Update ready. Restart to install.';
            primaryAction = { label: 'Restart Now', onClick: onInstall };
            secondaryActions = [{ label: 'Later', onClick: onDismiss }];
            break;
        case 'error':
            message = `Update error: ${state.message}`;
            secondaryActions = [{ label: 'Dismiss', onClick: onDismiss }];
            break;
    }

    return (
        <div className="update-notification" role="status" aria-live="polite">
            <span className="update-notification__message">{message}</span>
            <div className="update-notification__actions">
                {primaryAction && (
                    <button
                        className="update-notification__btn update-notification__btn--primary"
                        onClick={primaryAction.onClick}
                    >
                        {primaryAction.label}
                    </button>
                )}
                {secondaryActions.map((action) => (
                    <button
                        key={action.label}
                        className="update-notification__btn update-notification__btn--secondary"
                        onClick={action.onClick}
                    >
                        {action.label}
                    </button>
                ))}
            </div>
        </div>
    );
}
```

**Step 3: Run typecheck**

```bash
yarn typecheck
```

Expected: passes.

---

### Task 5: Integrate `UpdateNotification` into `App.tsx`

**Files:**

- Modify: `src/renderer/App.tsx`

**Step 1: Add imports**

At the top of `App.tsx`, add:

```typescript
import {
    UpdateNotification,
    type UpdateNotificationState,
} from './components/UpdateNotification';
import type { UpdateAvailableInfo } from '../shared/ipcTypes';
```

**Step 2: Add update state in the `App` function**

Near the other `useState` declarations at the top of the `App` function body, add:

```typescript
const [updateState, setUpdateState] = useState<UpdateNotificationState>({
    status: 'idle',
});
// Store the version currently being offered so we can reference it later
const pendingUpdateVersion = useRef<string>('');
```

**Step 3: Subscribe to update events**

In the existing `useEffect` block that handles subscriptions (or add a new one), add:

```typescript
useEffect(() => {
    const unsubAvailable = electronAPI.update.onAvailable(
        (info: UpdateAvailableInfo) => {
            pendingUpdateVersion.current = info.version;
            setUpdateState({
                status: 'available',
                version: info.version,
                releaseUrl: info.releaseUrl,
            });
        },
    );
    const unsubDownloading = electronAPI.update.onDownloading(() => {
        setUpdateState({
            status: 'downloading',
            version: pendingUpdateVersion.current,
        });
    });
    const unsubDownloaded = electronAPI.update.onDownloaded(() => {
        setUpdateState({ status: 'ready' });
    });
    const unsubError = electronAPI.update.onError((message: string) => {
        setUpdateState({ status: 'error', message });
    });

    return () => {
        unsubAvailable();
        unsubDownloading();
        unsubDownloaded();
        unsubError();
    };
}, []);
```

**Step 4: Add handler callbacks**

```typescript
const handleUpdateDownload = useCallback(() => {
    void electronAPI.update.download();
}, []);

const handleUpdateInstall = useCallback(() => {
    void electronAPI.update.install();
}, []);

const handleUpdateSkip = useCallback(() => {
    // Persist the skip to config so the banner doesn't reappear for this version
    if (pendingUpdateVersion.current) {
        void electronAPI.config.write({
            skippedUpdateVersion: pendingUpdateVersion.current,
        });
    }
    setUpdateState({ status: 'idle' });
}, []);

const handleUpdateDismiss = useCallback(() => {
    setUpdateState({ status: 'idle' });
}, []);
```

**Step 5: Render the component**

At the bottom of the JSX returned by `App`, just before the closing fragment/div, add:

```tsx
<UpdateNotification
    state={updateState}
    onDownload={handleUpdateDownload}
    onInstall={handleUpdateInstall}
    onSkip={handleUpdateSkip}
    onDismiss={handleUpdateDismiss}
/>
```

**Step 6: Run typecheck + lint**

```bash
yarn typecheck && yarn lint
```

Expected: passes.

**Step 7: Commit**

```bash
git add src/shared/ipcTypes.ts src/main/main.ts src/preload/preload.ts \
        src/renderer/components/UpdateNotification.tsx \
        src/renderer/components/UpdateNotification.css \
        src/renderer/App.tsx
git commit -m "feat: add opt-in in-app update notifications

Check for updates via GitHub API on startup; show dismissible banner
with explicit Download & Install action. Skip-version support persisted
to config. autoUpdater download only triggered by user consent."
```

---

## Notes

- **Linux**: `autoUpdater.checkForUpdates()` is gated behind `supportsAutoUpdater` check, so clicking "Download & Install" on Linux is a no-op in the main process. If you want Linux users to get a useful flow, after Task 2 you can add a fallback in the `UPDATE_DOWNLOAD` IPC handler to call `shell.openExternal(releaseUrl)` on Linux instead. Store the `releaseUrl` from `checkForUpdateAvailability` at module scope for this purpose.
- **Dev mode**: All update logic is gated behind `app.isPackaged`, so nothing runs in dev.
- **`update-electron-app` package**: It remains in `package.json` as an unused dependency. You can remove it after confirming everything works: `yarn remove update-electron-app`.
