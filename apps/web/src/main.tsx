import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
// Fontes auto-hospedadas (sem Google Fonts). A Bricolage vem com o eixo de tamanho óptico.
import '@fontsource-variable/bricolage-grotesque/opsz.css'
import '@fontsource-variable/atkinson-hyperlegible-next/wght.css'
import './index.css'
import App from './App.tsx'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
