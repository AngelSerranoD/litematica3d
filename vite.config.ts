// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import process from "node:process";
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [react()],

  build: {
    // Three.js ocupa ~740 kB minificado y no se puede partir; va aparte y se
    // carga bajo demanda. El entry (React, motion, zustand y la app) ronda los
    // 364 kB. Se sube el aviso a 800 kB para que solo salte si algo crece de
    // verdad por encima de lo ya conocido.
    chunkSizeWarningLimit: 800,
    rollupOptions: {
      output: {
        // Three.js va en su propio trozo: se pide al abrir un schematic y no
        // pesa en el arranque de la biblioteca.
        manualChunks: (id: string) =>
          id.includes("node_modules/three/") ? "three" : undefined,
      },
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
