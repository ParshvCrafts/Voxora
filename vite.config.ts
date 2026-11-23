import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";

// Two pages: the settings/history window and the floating overlay pill.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: {
    target: "es2022",
    rollupOptions: {
      input: { main: fileURLToPath(new URL("index.html", import.meta.url)), overlay: fileURLToPath(new URL("overlay.html", import.meta.url)) },
    },
  },
});
