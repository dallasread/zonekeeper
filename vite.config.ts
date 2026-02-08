import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { fileURLToPath } from "url";
import { dirname, resolve } from "path";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;
const __dirname = dirname(fileURLToPath(import.meta.url));

export default defineConfig(async () => ({
  plugins: [vue()],
  clearScreen: false,
  resolve: {
    alias: {
      // Force ESM entry (exports field only exposes CJS which causes readonly property errors)
      'isomorphic-git': resolve(__dirname, 'node_modules/isomorphic-git/index.js'),
    },
  },
  server: {
    port: 1421,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1422,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
}));
