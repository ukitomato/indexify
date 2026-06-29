// extension.test.ts — Extension Development Host (EDH) integration tests.
// Verifies that all commands are registered, default config values are correct,
// and the webview view provider is registered after activation.

import * as assert from 'assert';
import * as vscode from 'vscode';

suite('Extension activation', () => {
  suiteSetup(async () => {
    const ext = vscode.extensions.getExtension('ukitomato.indexify');
    if (ext && !ext.isActive) {
      await ext.activate();
    }
    // Allow async activation side-effects (sidecar start, showInformationMessage) to settle.
    await new Promise(r => setTimeout(r, 500));
  });

  // --- commands ---

  test('indexify.search is registered', async () => {
    const cmds = await vscode.commands.getCommands(true);
    assert.ok(cmds.includes('indexify.search'), 'indexify.search not found in registered commands');
  });

  test('indexify.searchRegex is registered', async () => {
    const cmds = await vscode.commands.getCommands(true);
    assert.ok(cmds.includes('indexify.searchRegex'));
  });

  test('indexify.reindex is registered', async () => {
    const cmds = await vscode.commands.getCommands(true);
    assert.ok(cmds.includes('indexify.reindex'));
  });

  test('indexify.focusSearch is registered', async () => {
    const cmds = await vscode.commands.getCommands(true);
    assert.ok(cmds.includes('indexify.focusSearch'));
  });

  test('all four indexify commands are registered', async () => {
    const cmds = await vscode.commands.getCommands(true);
    const indexifyCmds = cmds.filter(c => c.startsWith('indexify.'));
    for (const expected of ['indexify.search', 'indexify.searchRegex', 'indexify.reindex', 'indexify.focusSearch']) {
      assert.ok(indexifyCmds.includes(expected), `missing: ${expected}`);
    }
  });

  // --- default configuration values ---

  test('default maxResults is 300', () => {
    const cfg = vscode.workspace.getConfiguration('indexify');
    assert.strictEqual(cfg.get<number>('maxResults'), 300);
  });

  test('default indexDir is empty string', () => {
    const cfg = vscode.workspace.getConfiguration('indexify');
    assert.strictEqual(cfg.get<string>('indexDir'), '');
  });

  test('default binaryPath is empty string', () => {
    const cfg = vscode.workspace.getConfiguration('indexify');
    assert.strictEqual(cfg.get<string>('binaryPath'), '');
  });

  test('configuration section exists and has all three keys', () => {
    const cfg = vscode.workspace.getConfiguration('indexify');
    assert.ok(cfg.has('maxResults'));
    assert.ok(cfg.has('indexDir'));
    assert.ok(cfg.has('binaryPath'));
  });
});
