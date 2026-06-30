// esbuild.js — 拡張を単一の out/extension.js にバンドルする。
//
// external:
//   - vscode : provided by VS Code at runtime (cannot be bundled).
// The `loupe` native binary is spawned as a child process (NDJSON sidecar) — shipped under bin/,
// not via the JS bundle.

const esbuild = require('esbuild');

const production = process.argv.includes('--production');
const watch = process.argv.includes('--watch');

async function main() {
  const ctx = await esbuild.context({
    entryPoints: ['src/extension.ts'],
    bundle: true,
    format: 'cjs',
    platform: 'node',
    target: 'node18',
    outfile: 'out/extension.js',
    external: ['vscode'],
    sourcemap: !production,
    minify: production,
    logLevel: 'info',
  });
  if (watch) {
    await ctx.watch();
  } else {
    await ctx.rebuild();
    await ctx.dispose();
  }
}

main().catch((e) => { console.error(e); process.exit(1); });
