// config.test.ts — Tests for config.ts resolution logic.
// Runs in the Extension Development Host so vscode.workspace.* APIs are available.

import * as assert from 'assert';
import * as path from 'path';
import * as vscode from 'vscode';
import { indexDir, resolveBinary, setExtensionPath } from '../config';

function primaryRoot(): string {
  const ws = vscode.workspace.workspaceFolders;
  return ws && ws.length ? ws[0].uri.fsPath : process.cwd();
}

suite('Config: indexDir', () => {
  teardown(async () => {
    // Always restore the default (empty) after each test
    await vscode.workspace.getConfiguration('indexify')
      .update('indexDir', '', vscode.ConfigurationTarget.Global);
  });

  test('defaults to <workspace>/.indexify when setting is empty', () => {
    const dir = indexDir();
    const expected = path.join(primaryRoot(), '.indexify');
    assert.strictEqual(dir, expected);
  });

  test('uses the setting value when non-empty', async () => {
    const abs = process.platform === 'win32' ? 'C:\\custom\\idx' : '/custom/idx';
    await vscode.workspace.getConfiguration('indexify')
      .update('indexDir', abs, vscode.ConfigurationTarget.Global);
    assert.strictEqual(indexDir(), abs);
  });

  test('relative indexDir is resolved against workspace root', async () => {
    await vscode.workspace.getConfiguration('indexify')
      .update('indexDir', '.myindex', vscode.ConfigurationTarget.Global);
    const dir = indexDir();
    assert.ok(path.isAbsolute(dir), 'resolved path must be absolute');
    assert.ok(dir.endsWith('.myindex') || dir.endsWith('.myindex' + path.sep),
      `expected path ending in .myindex, got ${dir}`);
  });

  test('whitespace-only indexDir setting falls back to default', async () => {
    await vscode.workspace.getConfiguration('indexify')
      .update('indexDir', '   ', vscode.ConfigurationTarget.Global);
    const dir = indexDir();
    const expected = path.join(primaryRoot(), '.indexify');
    assert.strictEqual(dir, expected);
  });
});

suite('Config: resolveBinary', () => {
  teardown(async () => {
    await vscode.workspace.getConfiguration('indexify')
      .update('binaryPath', '', vscode.ConfigurationTarget.Global);
    setExtensionPath('');
  });

  test('falls back to "indexify" (PATH lookup) when no setting and no extensionPath', () => {
    setExtensionPath('');
    const bin = resolveBinary();
    assert.strictEqual(bin, 'indexify');
  });

  test('returns the binaryPath setting when non-empty', async () => {
    const custom = process.platform === 'win32' ? 'C:\\bin\\indexify.exe' : '/usr/local/bin/indexify';
    await vscode.workspace.getConfiguration('indexify')
      .update('binaryPath', custom, vscode.ConfigurationTarget.Global);
    setExtensionPath('');
    assert.strictEqual(resolveBinary(), custom);
  });

  test('binaryPath setting takes priority over extensionPath bundled binary', async () => {
    const custom = '/custom/indexify';
    await vscode.workspace.getConfiguration('indexify')
      .update('binaryPath', custom, vscode.ConfigurationTarget.Global);
    setExtensionPath('/some/extension/path');
    assert.strictEqual(resolveBinary(), custom);
  });

  test('whitespace-only binaryPath falls back to bundled / PATH', async () => {
    await vscode.workspace.getConfiguration('indexify')
      .update('binaryPath', '   ', vscode.ConfigurationTarget.Global);
    setExtensionPath('');
    assert.strictEqual(resolveBinary(), 'indexify');
  });

  test('returns a non-empty string in all cases', () => {
    setExtensionPath('');
    const bin = resolveBinary();
    assert.ok(bin.length > 0, 'resolveBinary must never return an empty string');
  });
});
