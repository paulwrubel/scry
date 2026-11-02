import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
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
  plugins: [react(), tailwindcss()],
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
