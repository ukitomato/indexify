// Compile TypeScript test files for the @vscode/test-cli runner.
// Uses esbuild (fast, no type checking — tsc --noEmit runs first via pretest).
// Each test file is bundled independently so relative imports (../../config etc.) resolve.

const path = require('path');
const fs = require('fs');
const esbuild = require('esbuild');

const testDir = 'src/test';
const files = fs.readdirSync(testDir)
  .filter(f => f.endsWith('.test.ts'))
  .map(f => path.join(testDir, f));

if (files.length === 0) {
  console.error('No *.test.ts files found in', testDir);
  process.exit(1);
}

esbuild.build({
  entryPoints: files,
  bundle: true,
  format: 'cjs',
  platform: 'node',
  target: 'node18',
  outdir: 'out/test',
  outbase: testDir,
  external: ['vscode'],
  sourcemap: true,
  logLevel: 'info',
}).catch(e => {
  console.error(e);
  process.exit(1);
});
