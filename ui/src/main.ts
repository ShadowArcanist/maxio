import { mount } from 'svelte'
import './app.css'
import App from './App.svelte'

// Restore theme preference before mount to avoid flash
const saved = localStorage.getItem('theme')
if (saved === 'light') {
  document.documentElement.classList.remove('dark')
} else {
  document.documentElement.classList.add('dark')
}

const app = mount(App, {
  target: document.getElementById('app')!,
})

export default app
