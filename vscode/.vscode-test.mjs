import { defineConfig } from '@vscode/test-cli';

export default defineConfig({
  files: 'out/test/**/*.test.js',
  workspaceFolder: 'src/test/fixtures/workspace',
  mocha: {
    timeout: 20000,
    ui: 'tdd',
  },
});
