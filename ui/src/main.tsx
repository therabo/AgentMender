import { createRoot } from 'react-dom/client'
import { App } from './app/App'
import './styles/tokens.css'
import './styles/global.css'
import './styles/inventory.css'
import './styles/preview.css'
import './styles/diagnostics.css'
import './styles/motion.css'

createRoot(document.getElementById('root')!).render(<App />)
