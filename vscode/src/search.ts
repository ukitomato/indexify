// search.ts — query the sidecar and render results in a progressive QuickPick.

import * as vscode from 'vscode';
import * as path from 'path';
import { cfg } from './config';
import { openMatch } from './openMatch';
import type { Match, Sidecar } from './sidecarClient';

interface ResultItem extends vscode.QuickPickItem {
  _file: string;
  _line: number;
}

function toItem(m: Match): ResultItem {
  return {
    label: `$(symbol-file) ${path.basename(m.file)}:${m.line}`,
    description: m.text,
    detail: m.file,
    _file: m.file,
    _line: m.line,
  };
}

export async function doSearch(sidecar: Sidecar, regex: boolean): Promise<void> {
  const editor = vscode.window.activeTextEditor;
  const seed = editor && !editor.selection.isEmpty ? editor.document.getText(editor.selection) : '';
  const query = await vscode.window.showInputBox({
    prompt: regex ? 'indexify (regex)' : 'indexify (substring)',
    value: seed,
    placeHolder: regex ? 'e.g. func\\s+\\w+ (needs a >=3-char literal)' : 'e.g. calcTotal',
  });
  if (!query) {
    return;
  }
  const max = cfg().get<number>('maxResults') ?? 300;

  const qp = vscode.window.createQuickPick<ResultItem>();
  qp.matchOnDescription = true;
  qp.matchOnDetail = true;
  qp.busy = true;
  qp.placeholder = `Searching "${query}"…`;

  let cancelled = false;
  const items: ResultItem[] = [];
  let flushTimer: NodeJS.Timeout | null = null;
  const flush = () => {
    flushTimer = null;
    qp.items = items.slice();
  };
  const scheduleFlush = () => {
    if (!flushTimer) {
      flushTimer = setTimeout(flush, 80);
    }
  };

  qp.onDidAccept(() => {
    const pick = qp.selectedItems[0];
    qp.hide();
    if (pick) {
      void openMatch(pick._file, pick._line);
    }
  });
  qp.onDidHide(() => {
    cancelled = true;
    if (flushTimer) {
      clearTimeout(flushTimer);
    }
    qp.dispose();
  });
  qp.show();

  try {
    const { hits } = await sidecar.search(query, regex, max, false, (m) => {
      if (!cancelled && items.length < max) {
        items.push(toItem(m));
        scheduleFlush();
      }
    });
    if (!cancelled) {
      if (flushTimer) {
        clearTimeout(flushTimer);
      }
      qp.items = items.slice();
      qp.busy = false;
      qp.placeholder = hits ? `${hits} results: "${query}"` : `No matches: "${query}"`;
    }
  } catch (e: any) {
    if (!cancelled) {
      qp.busy = false;
      qp.placeholder = `Error: ${e?.message ?? e}`;
      vscode.window.showErrorMessage(`indexify failed: ${e?.message ?? e}`);
    }
  }
}
