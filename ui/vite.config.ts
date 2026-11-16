import { defineConfig } from 'vite'

export default defineConfig({
  build: {
    cssCodeSplit: false,
    rollupOptions: {
      output: {
        entryFileNames: 'app.js',
        assetFileNames: (asset) => asset.names.includes('style.css') ? 'app.css' : '[name][extname]',
      },
    },
  },
})
