import { defineConfig } from 'vite'

// Plugin to watch WASM build output and trigger full page reload
function wasmHotReload() {
  return {
    name: 'wasm-hot-reload',
    configureServer(server) {
      // Watch the pkg/ directory for changes
      server.watcher.add('./pkg/*.wasm');
      server.watcher.add('./pkg/*.js');

      server.watcher.on('change', (path) => {
        if (path.includes('/pkg/')) {
          console.log('[WASM] Detected change, reloading...');
          // Trigger full page reload for WASM changes
          server.ws.send({ type: 'full-reload' });
        }
      });
    }
  };
}

export default defineConfig({
  plugins: [wasmHotReload()],
  build: {
    outDir: 'dist',
    sourcemap: true
  }
})
