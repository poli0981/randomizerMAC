import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Tauri expects a fixed port; TAURI_DEV_HOST is set when developing on a remote device.
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**", "**/crates/**", "**/target/**"] },
  },
  build: {
    target: "es2022",
    sourcemap: false,
    chunkSizeWarningLimit: 800,
  },
  resolve: process.env.VITEST ? { conditions: ["browser"] } : undefined,
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.ts"],
    // `npm run test:coverage` → coverage/lcov.info, imported by SonarQube Cloud (.svelte files
    // are not analyzed there, so only TypeScript modules are measured).
    coverage: {
      provider: "v8",
      include: ["src/**/*.ts"],
      exclude: ["src/**/*.test.ts", "src/**/*.d.ts"],
      reporter: ["text-summary", "lcov"],
    },
  },
});
