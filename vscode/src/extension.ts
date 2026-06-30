// extension.ts — lifecycle, sidecar management, and commands.

import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { doSearch } from './search';
import { indexDir, resolveBinary, setExtensionPath } from './config';
import { Sidecar } from './sidecarClient';
import { SearchViewProvider } from './searchView';

let sidecar: Sidecar | undefined;

/** True once the Tantivy index has been committed at least once (it writes tantivy/meta.json). */
function indexExists(dir: string): boolean {
  try {
    return fs.existsSync(path.join(dir, 'tantivy', 'meta.json'));
  } catch {
    return false;
  }
}

async function reindex(): Promise<void> {
  if (!sidecar) {
    return;
  }
  await vscode.window.withProgress(
    { location: vscode.ProgressLocation.Notification, title: 'loupe: indexing…', cancellable: false },
    async (progress) => {
      try {
        const res = await sidecar!.sync((indexed, message) => {
          progress.report({ message: message ?? `${indexed.toLocaleString()} files…` });
        });
        vscode.window.showInformationMessage(
          `loupe: index up to date (${res.updated.toLocaleString()} updated, ${res.removed.toLocaleString()} removed, ${(res.ms / 1000).toFixed(1)}s).`
        );
      } catch (e: any) {
        vscode.window.showErrorMessage(`loupe: indexing failed: ${e?.message ?? e}`);
      }
    }
  );
}

export function activate(context: vscode.ExtensionContext): void {
  setExtensionPath(context.extensionPath);
  const dir = indexDir();
  fs.mkdirSync(dir, { recursive: true });
  sidecar = new Sidecar(resolveBinary(), dir);
  sidecar.start();

  // If an index already exists, run a background catch-up sync (reindex only changed/new/deleted
  // files by mtime) which also (re)starts the incremental watcher. Cheap when little changed.
  if (indexExists(dir)) {
    void sidecar.sync(() => {/* silent */}).catch(() => {/* ignore */});
  } else {
    vscode.window
      .showInformationMessage('loupe: no index yet. Build it now?', 'Build', 'Later')
      .then((sel) => {
        if (sel === 'Build') {
          void reindex();
        }
      });
  }

  const searchView = new SearchViewProvider(context.extensionUri, () => sidecar);

  context.subscriptions.push(
    vscode.window.registerWebviewViewProvider(SearchViewProvider.viewType, searchView, {
      webviewOptions: { retainContextWhenHidden: true },
    }),
    vscode.commands.registerCommand('loupe.search', () => sidecar && doSearch(sidecar, false)),
    vscode.commands.registerCommand('loupe.searchRegex', () => sidecar && doSearch(sidecar, true)),
    vscode.commands.registerCommand('loupe.reindex', () => reindex()),
    vscode.commands.registerCommand('loupe.focusSearch', async () => {
      await vscode.commands.executeCommand('loupe.searchView.focus');
      searchView.focus();
    }),
    { dispose: () => sidecar?.dispose() }
  );
}

export function deactivate(): void {
  sidecar?.dispose();
  sidecar = undefined;
}
