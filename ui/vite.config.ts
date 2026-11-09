import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import { VitePWA } from 'vite-plugin-pwa';
import { execSync } from 'node:child_process';

function getVersion(): string {
  const desc = execSync('git describe --tags --always --dirty --abbrev=7').toString().trim();
  const hash = execSync('git rev-parse --short=7 HEAD').toString().trim();

  // only possible in a local environment
  if (desc.includes('-dirty')) {
    return `local-${hash}`;
  }

  // clean semver tag (desc matches exactly, no extra commits past the tag)
  if (/^v?\d+\.\d+\.\d+$/.test(desc)) {
    return desc;
  }

  return hash;
}

export default defineConfig({
  define: {
    __APP_VERSION__: JSON.stringify(getVersion()),
  },
  plugins: [
    react(),
    tailwindcss(),
    VitePWA({
      registerType: 'autoUpdate',
      includeAssets: ['favicon.svg', 'favicon-32x32.png', 'favicon.ico', 'apple-touch-icon.png'],
      manifest: {
        name: 'Scry',
        short_name: 'Scry',
        description: 'A local-first task manager with a CLI and an offline web UI',
        id: '/',
        start_url: '/',
        scope: '/',
        display: 'standalone',
        background_color: '#18181b',
        theme_color: '#18181b',
        icons: [
          { src: '/pwa-192x192.png', sizes: '192x192', type: 'image/png' },
          { src: '/pwa-512x512.png', sizes: '512x512', type: 'image/png' },
          {
            src: '/pwa-maskable-512x512.png',
            sizes: '512x512',
            type: 'image/png',
            purpose: 'maskable',
          },
        ],
      },
      workbox: {
        globPatterns: ['**/*.{js,css,html,svg,png,webmanifest}'],
        navigateFallback: '/index.html',
        navigateFallbackDenylist: [/^\/api\//, /^\/sync($|\/)/],
        cleanupOutdatedCaches: true,
      },
      devOptions: { enabled: false },
    }),
  ],
  resolve: {
    alias: {
      '@': '/src',
    },
  },
  server: {
    port: 5173,
    proxy: {
      '/api': 'http://localhost:8000',
      '/sync': 'http://localhost:8000',
      '/auth': 'http://localhost:8000',
    },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
});
