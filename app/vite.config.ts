import path from "path"
import react from "@vitejs/plugin-react"
import { defineConfig } from "vite"
import basicSsl from '@vitejs/plugin-basic-ssl'

// https://vite.dev/config/
export default defineConfig({
  base: './',
  plugins: [react(), basicSsl()],
  server: {
    port: 8080,
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
});
