import { useEffect, useLayoutEffect, useState, useSyncExternalStore } from 'react'
import { Link, Outlet, useLocation } from 'react-router'
import { Logo } from './Logo'
import { ServerWakingBanner } from './ui'
import { warmUp } from '../lib/api'
import { historyAvailable } from '../lib/history'

const LARGE_TEXT_KEY = 'humanobill:texto-grande'
const LARGE_TEXT_CLASS = 'texto-grande' // definida em index.css: sobe a base para 125%

/** O localStorage pode estar bloqueado (modo privado, cookies desligados): sem ele, vale o tamanho normal. */
function readLargeText(): boolean {
  try {
    return window.localStorage.getItem(LARGE_TEXT_KEY) === '1'
  } catch {
    return false
  }
}

/** Modo A+ (RF20): liga a classe em <html> e guarda a escolha. */
function useLargeText(): [boolean, () => void] {
  const [on, setOn] = useState(readLargeText)
  // Antes da pintura, para quem já escolheu A+ não ver o texto "pular" de tamanho.
  useLayoutEffect(() => {
    document.documentElement.classList.toggle(LARGE_TEXT_CLASS, on)
  }, [on])
  const toggle = () => {
    const next = !on
    setOn(next)
    try {
      window.localStorage.setItem(LARGE_TEXT_KEY, next ? '1' : '0')
    } catch {
      /* sem localStorage a escolha vale só até recarregar */
    }
  }
  return [on, toggle]
}

// Mesmo corte do `sm` do Tailwind. Abaixo dele, os links ficam só no rodapé (guia 5.4).
const NAV_LINKS_QUERY = '(min-width: 40rem)'
const subscribeNavLinks = (onChange: () => void) => {
  const mq = window.matchMedia?.(NAV_LINKS_QUERY)
  mq?.addEventListener('change', onChange)
  return () => mq?.removeEventListener('change', onChange)
}
const showNavLinks = () => window.matchMedia?.(NAV_LINKS_QUERY).matches ?? false

const NAV_LINKS = [
  { to: '/#como-funciona', label: 'Como funciona' },
  { to: '/minhas-contas', label: 'Minhas contas' },
  { to: '/privacidade', label: 'Privacidade' },
]
// Sem localStorage a lista de contas não funciona, então o link nem aparece.
const NAV_LINKS_SEM_HISTORICO = NAV_LINKS.filter((l) => l.to !== '/minhas-contas')
// O painel coletivo (#62) é discreto: só no rodapé, para não disputar espaço na navegação.
const PANEL_LINK = { to: '/painel', label: 'O que já encontramos' }

export default function Layout() {
  const { pathname, hash, key } = useLocation()
  const [largeText, toggleLargeText] = useLargeText()
  // Os links da navbar saem do DOM (e não só da tela) no celular: assim não ficam
  // duplicados com os do rodapé para quem usa leitor de tela.
  const navLinks = useSyncExternalStore(subscribeNavLinks, showNavLinks)
  const [links] = useState(() => (historyAvailable() ? NAV_LINKS : NAV_LINKS_SEM_HISTORICO))

  useEffect(() => {
    void warmUp()
  }, [])
  useEffect(() => {
    window.scrollTo(0, 0)
  }, [pathname])
  // Links com âncora (ex.: "Como funciona"): rola até o alvo e leva o foco a ele, se aceitar foco.
  useEffect(() => {
    if (!hash) return
    const target = document.getElementById(decodeURIComponent(hash.slice(1)))
    if (!target) return
    target.scrollIntoView?.({ block: 'start' })
    target.focus({ preventScroll: true })
  }, [hash, key])

  return (
    <div className="flex min-h-dvh flex-col bg-bg text-ink">
      <a href="#conteudo" className="sr-only font-semibold text-accent underline focus:not-sr-only focus:block focus:bg-surface focus:px-5 focus:py-3">
        Ir para o conteúdo
      </a>
      <ServerWakingBanner />
      <header className="bg-accent text-on-accent">
        <div className="mx-auto flex min-h-16 max-w-page flex-wrap items-center gap-x-2 px-5 md:px-10">
          <Link to="/" className="foco-sobre-azul mr-auto inline-flex min-h-11.5 items-center rounded-control">
            <Logo negative />
          </Link>
          {navLinks && (
            <nav aria-label="Principal" className="flex items-center gap-2">
              {links.map((l) => (
                <Link
                  key={l.to}
                  to={l.to}
                  className="foco-sobre-azul inline-flex min-h-11.5 items-center rounded-control px-3 font-semibold underline-offset-4 hover:underline"
                >
                  {l.label}
                </Link>
              ))}
            </nav>
          )}
          <button
            type="button"
            onClick={toggleLargeText}
            aria-pressed={largeText}
            aria-label="Aumentar o texto"
            className={`foco-sobre-azul transicao-controle grid size-[2.8rem] shrink-0 place-items-center rounded-control border-[1.5px] border-on-accent font-bold ${
              largeText ? 'bg-on-accent text-accent' : 'text-on-accent hover:bg-on-accent/15'
            }`}
          >
            A+
          </button>
        </div>
      </header>
      <main id="conteudo" className="flex-1">
        <Outlet />
      </main>
      <footer className="border-t border-line bg-bg text-caption text-muted">
        <div className="mx-auto flex max-w-page flex-col gap-x-6 px-5 py-4 sm:flex-row sm:items-center sm:justify-between md:px-10">
          <p className="py-2">Conteúdo informativo, não é orientação jurídica.</p>
          <nav aria-label="Rodapé" className="flex flex-wrap gap-x-6">
            {[...links, PANEL_LINK].map((l) => (
              <Link key={l.to} to={l.to} className="inline-flex min-h-11.5 items-center underline underline-offset-4">
                {l.label}
              </Link>
            ))}
          </nav>
        </div>
      </footer>
    </div>
  )
}
