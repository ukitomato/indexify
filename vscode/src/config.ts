// config.ts — settings access and binary/path resolution (project-agnostic).

import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

export function cfg(): vscode.WorkspaceConfiguration {
  return vscode.workspace.getConfiguration('indexify');
}

let extensionPath = '';
export function setExtensionPath(p: string): void {
  extensionPath = p;
}

/** The first workspace folder (used as the default root and to locate the index dir). */
export function primaryRoot(): string {
  const ws = vscode.workspace.workspaceFolders;
  return ws && ws.length ? ws[0].uri.fsPath : process.cwd();
}

function toAbs(p: string): string {
  return path.isAbsolute(p) ? p : path.resolve(primaryRoot(), p);
}

/**
 * Index directory, shared with the CLI and MCP server. `indexify.indexDir` overrides; otherwise
 * `<workspace>/.indexify`. Keeping all three front-ends on one directory means a single build is
 * reused everywhere.
 */
export function indexDir(): string {
  const c = (cfg().get<string>('indexDir') ?? '').trim();
  return c ? toAbs(c) : path.join(primaryRoot(), '.indexify');
}

/** Platform subdir + filename of the bundled binary, e.g. linux-x64/indexify. */
function bundledBinaryRelPath(): string {
  const exe = process.platform === 'win32' ? 'indexify.exe' : 'indexify';
  const arch = process.arch === 'arm64' ? 'arm64' : 'x64';
  const os =
    process.platform === 'win32' ? 'win32' : process.platform === 'darwin' ? 'darwin' : 'linux';
  return path.join('bin', `${os}-${arch}`, exe);
}

/** Resolve the indexify binary: setting → bundled binary for this platform → 'indexify' on PATH. */
export function resolveBinary(): string {
  const c = (cfg().get<string>('binaryPath') ?? '').trim();
  if (c) {
    return c;
  }
  if (extensionPath) {
    const rel = bundledBinaryRelPath();
    // Packaged .vsix copies bin/ next to the extension; in the repo the product-level bin/ is one
    // level up (tools/indexify/bin), shared with the CLI and MCP server.
    for (const base of [extensionPath, path.join(extensionPath, '..')]) {
      const p = path.join(base, rel);
      if (fs.existsSync(p)) {
        return p;
      }
    }
  }
  return 'indexify';
}
