import { defineConfig } from 'vite'

export default defineConfig({
  build: {
    cssCodeSplit: false,
    assetsInlineLimit: 0,
    rollupOptions: {
      output: {
        entryFileNames: 'app.js',
        assetFileNames: (asset) => asset.names.some(name => name.endsWith('.css')) ? 'app.css' : 'assets/[name]-[hash][extname]',
      },
    },
  },
})
