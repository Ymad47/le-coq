import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import sveltePreprocess from 'svelte-preprocess';

// The API key is added by the proxy (server side), never shipped in the JS bundle.
// Anyone who can reach this dev server gets authenticated API access, so it
// listens on loopback by default. Override with LE_COQ_UI_HOST=0.0.0.0 only on a
// trusted network (e.g. a WireGuard interface).
const backend = process.env.LE_COQ_BACKEND_URL ?? 'http://127.0.0.1:8000';
const apiKey = process.env.LE_COQ_API_KEY;
const host = process.env.LE_COQ_UI_HOST ?? '127.0.0.1';

const target = {
  target: backend,
  headers: apiKey ? { 'X-API-Key': apiKey } : {},
};
const proxy = { '/api': target, '/health': target };

export default defineConfig({
  plugins: [
    svelte({
      preprocess: sveltePreprocess({ typescript: true }),
    }),
  ],
  server: { host, port: 3001, strictPort: true, proxy },
  preview: { host, port: 3001, strictPort: true, proxy },
});
