import { defineConfig } from 'vite';
import { resolve } from 'node:path';

const pages = [
  'index.html',
  'product/index.html',
  'how-it-works/index.html',
  'research/index.html',
  'compare/index.html',
  'journal/index.html',
  'journal/one-worker/index.html',
  'journal/demand-shape/index.html',
  'journal/supervision/index.html'
];

export default defineConfig({
  build: {
    rollupOptions: {
      input: Object.fromEntries(pages.map((page) => [page, resolve(import.meta.dirname, page)]))
    }
  }
});
