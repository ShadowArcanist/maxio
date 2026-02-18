import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import tailwindcss from '@tailwindcss/vite'
import path from 'path'

export default defineConfig({
  plugins: [svelte(), tailwindcss()],
  base: '/ui/',
  resolve: {
    alias: {
      $lib: path.resolve('./src/lib'),
    },
  },
})
