import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// The Python backend (tools/map_editor --web) listens on 8017 by default;
// during `npm run dev` the Vite dev server proxies API calls to it.
export default defineConfig({
  plugins: [react()],
  server: {
    proxy: {
      "/api": "http://127.0.0.1:8017",
    },
  },
});
