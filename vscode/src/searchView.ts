// searchView.ts — a persistent sidebar search view (Activity Bar), rendered as a Webview so it can
// embed a search box and a grouped, clickable result list like the native Search view. It reuses the
// same streaming sidecar (Sidecar.search) and the same open-at-line helper as the QuickPick.
//
// stdout/protocol notes:
//   webview -> ext : { type: "search", query, regex } | { type: "open", file, line, column }
//   ext -> webview : { type: "begin", query } | { type: "result", file, line, text }
//                    { type: "done", hits, tooShort? } | { type: "error", message } | { type: "focus" }
// A generation counter drops results from superseded queries (the daemon streams a whole query, and
// the user may type faster than that).

import * as vscode from 'vscode';
import { cfg, primaryRoot } from './config';
import { openMatch } from './openMatch';
import type { Sidecar } from './sidecarClient';

export class SearchViewProvider implements vscode.WebviewViewProvider {
  public static readonly viewType = 'loupe.searchView';
  private view?: vscode.WebviewView;
  private gen = 0;

  constructor(
    private readonly extensionUri: vscode.Uri,
    private readonly getSidecar: () => Sidecar | undefined,
  ) {}

  resolveWebviewView(view: vscode.WebviewView): void {
    this.view = view;
    view.webview.options = {
      enableScripts: true,
      localResourceRoots: [vscode.Uri.joinPath(this.extensionUri, 'media')],
    };
    view.webview.html = this.html(view.webview);
    view.webview.onDidReceiveMessage((msg) => void this.onMessage(msg));
  }

  /** Reveal the view and focus its input (bound to the loupe.focusSearch command). */
  focus(): void {
    this.view?.show?.(true);
    this.view?.webview.postMessage({ type: 'focus' });
  }

  private async onMessage(msg: any): Promise<void> {
    if (!msg) {
      return;
    }
    if (msg.type === 'open') {
      await openMatch(String(msg.file), Number(msg.line) || 1, Number(msg.column) || 0);
    } else if (msg.type === 'search') {
      const raw = typeof msg.max === 'number' ? msg.max : (cfg().get<number>('maxResults') ?? 300);
      // 0 means "unlimited" in the UI; pass a large number to the sidecar (bounded internally by
      // CANDIDATE_LIMIT × PER_FILE_MATCH_CAP in the Rust searcher, so memory is still safe).
      const max = raw === 0 ? 999999 : raw;
      await this.runSearch(String(msg.query ?? ''), !!msg.regex, !!msg.caseSensitive, max);
    }
  }

  private async runSearch(query: string, regex: boolean, caseSensitive: boolean, max: number): Promise<void> {
    const view = this.view;
    if (!view) {
      return;
    }
    const myGen = ++this.gen;
    const post = (m: any) => {
      if (myGen === this.gen) {
        void view.webview.postMessage(m);
      }
    };
    post({ type: 'begin', query });
    if (query.length < 3) {
      post({ type: 'done', hits: 0, tooShort: query.length > 0 });
      return;
    }
    const sidecar = this.getSidecar();
    if (!sidecar) {
      post({ type: 'error', message: 'loupe is not running yet' });
      return;
    }
    try {
      const { hits } = await sidecar.search(query, regex, max, caseSensitive, (m) => {
        post({ type: 'result', file: m.file, line: m.line, text: m.text });
      });
      post({ type: 'done', hits });
    } catch (e: any) {
      post({ type: 'error', message: e?.message ?? String(e) });
    }
  }

  private html(webview: vscode.Webview): string {
    const nonce = makeNonce();
    const cssUri = webview.asWebviewUri(vscode.Uri.joinPath(this.extensionUri, 'media', 'searchView.css'));
    const jsUri = webview.asWebviewUri(vscode.Uri.joinPath(this.extensionUri, 'media', 'searchView.js'));
    const csp = [
      `default-src 'none'`,
      `style-src ${webview.cspSource}`,
      `script-src 'nonce-${nonce}'`,
    ].join('; ');
    const root = primaryRoot().replace(/"/g, '&quot;');
    const maxResults = cfg().get<number>('maxResults') ?? 300;
    return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta http-equiv="Content-Security-Policy" content="${csp}" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <link href="${cssUri}" rel="stylesheet" />
</head>
<body data-root="${root}" data-max="${maxResults}">
  <div class="toolbar">
    <div class="toolbar-row">
      <div class="inputwrap">
        <input id="q" type="text" placeholder="Search" autocomplete="off" spellcheck="false" />
        <button id="cs" class="iconbtn toggle" title="Match Case">Aa</button>
        <button id="rx" class="iconbtn toggle" title="Use Regular Expression (needs a ≥3-char literal)">.*</button>
      </div>
    </div>
    <div class="toolbar-row toolbar-row--between">
      <button id="maxBtn" class="max-btn" title="Max results — click to cycle: 50 / 100 / 300 / 1000 / ∞">max 300 ▾</button>
      <button id="toggle-filters" class="iconbtn toggle" title="Toggle path filters (include / exclude)">···</button>
    </div>
    <div id="filter-rows" hidden>
      <div class="toolbar-row">
        <div class="inputwrap">
          <input id="pathFilter" type="text" placeholder="files to include (e.g. src/, *.java)" autocomplete="off" spellcheck="false" />
        </div>
      </div>
      <div class="toolbar-row">
        <div class="inputwrap">
          <input id="excludeFilter" type="text" placeholder="files to exclude (e.g. *.min.js, test/)" autocomplete="off" spellcheck="false" />
        </div>
      </div>
    </div>
  </div>
  <div id="status" class="status"></div>
  <div id="results" class="results" tabindex="0"></div>
  <script nonce="${nonce}" src="${jsUri}"></script>
</body>
</html>`;
  }
}

function makeNonce(): string {
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
  let s = '';
  for (let i = 0; i < 32; i++) {
    s += chars.charAt(Math.floor(Math.random() * chars.length));
  }
  return s;
}
