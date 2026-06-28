// openMatch.ts — open a file at a specific line/column. Shared by the QuickPick (search.ts) and the
// sidebar search view (searchView.ts) so both jump to results the same way.

import * as vscode from 'vscode';

/** Open `file` and put the cursor at a 1-based `line` / 0-based `column`, revealing it centered. */
export async function openMatch(file: string, line: number, column = 0): Promise<void> {
  try {
    const doc = await vscode.workspace.openTextDocument(file);
    const ed = await vscode.window.showTextDocument(doc);
    const pos = new vscode.Position(Math.max(0, (line || 1) - 1), Math.max(0, column));
    ed.selection = new vscode.Selection(pos, pos);
    ed.revealRange(new vscode.Range(pos, pos), vscode.TextEditorRevealType.InCenter);
  } catch (e: any) {
    vscode.window.showErrorMessage(`Could not open: ${file} (${e?.message ?? e})`);
  }
}
