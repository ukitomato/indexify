// sidecarClient.ts — manage the indexify daemon (NDJSON over stdio) and correlate requests.
//
// Spawns `indexify serve --index-dir <indexDir>`, keeps it running, and routes responses by request
// id. search() streams matches via onMatch; build() reports progress via onProgress.

import { spawn, type ChildProcess } from 'child_process';
import * as readline from 'readline';

export interface Match {
  file: string;
  line: number;
  text: string;
}
type MatchCb = (m: Match) => void;
type ProgressCb = (indexed: number, message?: string) => void;

interface Pending {
  onMatch?: MatchCb;
  onProgress?: ProgressCb;
  resolve: (v: any) => void;
  reject: (e: any) => void;
}

export class Sidecar {
  private proc?: ChildProcess;
  private rl?: readline.Interface;
  private nextId = 1;
  private pending = new Map<number, Pending>();
  private ready!: Promise<void>;
  private readyResolve?: () => void;
  private started = false;

  constructor(private exePath: string, private indexDir: string) {}

  start(): void {
    if (this.started) {
      return;
    }
    this.started = true;
    this.ready = new Promise((r) => (this.readyResolve = r));
    this.proc = spawn(this.exePath, ['serve', '--index-dir', this.indexDir], { windowsHide: true });
    this.proc.on('error', (e) => this.failAll(e));
    this.proc.on('exit', () => this.failAll(new Error('sidecar exited')));
    this.rl = readline.createInterface({ input: this.proc.stdout! });
    this.rl.on('line', (line) => this.onLine(line));
  }

  private failAll(err: Error): void {
    for (const [, p] of this.pending) {
      p.reject(err);
    }
    this.pending.clear();
  }

  private onLine(line: string): void {
    if (!line.trim()) {
      return;
    }
    let msg: any;
    try {
      msg = JSON.parse(line);
    } catch {
      return;
    }
    if (msg.type === 'ready') {
      this.readyResolve?.();
      return;
    }
    // top-level error with no id → fail everything in flight
    if (msg.type === 'error' && (msg.id === undefined || msg.id === null)) {
      this.failAll(new Error(msg.message || 'sidecar error'));
      return;
    }
    const h = this.pending.get(msg.id);
    if (!h) {
      return;
    }
    switch (msg.type) {
      case 'match':
        h.onMatch?.({ file: msg.file, line: msg.line, text: msg.text });
        break;
      case 'done':
        this.pending.delete(msg.id);
        h.resolve({ hits: msg.hits });
        break;
      case 'progress':
        h.onProgress?.(msg.indexed ?? 0, msg.message);
        break;
      case 'built':
        this.pending.delete(msg.id);
        h.resolve({ files: msg.files, ms: msg.ms, attempts: msg.attempts });
        break;
      case 'watching':
        this.pending.delete(msg.id);
        h.resolve({ watching: true });
        break;
      case 'synced':
        this.pending.delete(msg.id);
        h.resolve({ updated: msg.updated, removed: msg.removed, ms: msg.ms });
        break;
      case 'error':
        this.pending.delete(msg.id);
        h.reject(new Error(msg.message || 'error'));
        break;
    }
  }

  private send(obj: any): void {
    if (!this.proc?.stdin?.writable) {
      throw new Error('sidecar not running');
    }
    this.proc.stdin.write(JSON.stringify(obj) + '\n');
  }

  async search(query: string, regex: boolean, max: number, caseSensitive: boolean, onMatch: MatchCb): Promise<{ hits: number }> {
    await this.ready;
    const id = this.nextId++;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { onMatch, resolve, reject });
      this.send({ id, cmd: 'search', query, regex, max, caseSensitive });
    });
  }

  // Roots aren't passed here: the binary reads them from settings.json (the shared source of
  // truth), so the extension can't disagree with the CLI/MCP about what to index.
  async build(onProgress: ProgressCb): Promise<{ files: number; ms: number; attempts: number }> {
    await this.ready;
    const id = this.nextId++;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { onProgress, resolve, reject });
      this.send({ id, cmd: 'build' });
    });
  }

  async sync(onProgress: ProgressCb): Promise<{ updated: number; removed: number; ms: number }> {
    await this.ready;
    const id = this.nextId++;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { onProgress, resolve, reject });
      this.send({ id, cmd: 'sync' });
    });
  }

  async watch(): Promise<void> {
    await this.ready;
    const id = this.nextId++;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.send({ id, cmd: 'watch' });
    });
  }

  dispose(): void {
    try {
      this.send({ id: this.nextId++, cmd: 'stop' });
    } catch {
      /* ignore */
    }
    this.rl?.close();
    this.proc?.kill();
    this.proc = undefined;
    this.started = false;
  }
}
