import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [sveltekit()],
  resolve: {
    /* The shared library lives outside this package; one copy of each runtime dependency. */
    dedupe: ["svelte", "@lucide/svelte"],
  },
  server: { fs: { allow: [".."] } },
});
