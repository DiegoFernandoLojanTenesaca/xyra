import { defineConfig } from 'vite';
import { svelte, vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';

const path = (folder) => fileURLToPath(new URL(folder, import.meta.url));

/** The phone app: its own entry in mobile/, sharing the design tokens, texts, types and some parts of the desktop UI. */
export default defineConfig({
  root: path('./mobile'),
  plugins: [svelte({ configFile: false, preprocess: vitePreprocess() })],
  resolve: { alias: { $shared: path('./src/lib'), $design: path('./design'), $locales: path('./locales') } },
  clearScreen: false,
  server: { port: 1430, strictPort: true, host: '127.0.0.1' },
  build: { outDir: path('./mobile/dist'), emptyOutDir: true },
});
