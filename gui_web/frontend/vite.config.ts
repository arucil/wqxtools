import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import zipPack from "vite-plugin-zip-pack";

export default defineConfig({
  plugins: [
    svelte(),
    zipPack({
      outDir: __dirname,
      outFileName: "frontend.zip",
    }),
  ],
});
