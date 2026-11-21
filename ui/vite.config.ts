import { readFileSync } from 'node:fs'
import { defineConfig } from 'vite'

const threeLicense = readFileSync(new URL('./node_modules/three/LICENSE', import.meta.url), 'utf8').trim()

export default defineConfig({
  esbuild: { legalComments: 'inline' },
  build: {
    cssCodeSplit: false,
    assetsInlineLimit: 0,
    rollupOptions: {
      output: {
        banner: chunk => chunk.moduleIds.some(id => id.includes('/node_modules/three/')) ? `/*! ${threeLicense} */` : '',
        entryFileNames: 'app.js',
        assetFileNames: (asset) => asset.names.some(name => name.endsWith('.css')) ? 'app.css' : 'assets/[name]-[hash][extname]',
      },
    },
  },
})
