import { useEffect } from 'react'
import { Link, Outlet, useLocation } from 'react-router'
import { ServerWakingBanner } from './ui'
import { warmUp } from '../lib/api'

export default function Layout() {
  const { pathname } = useLocation()
  useEffect(() => {
    void warmUp()
  }, [])
  useEffect(() => {
    window.scrollTo(0, 0)
  }, [pathname])

  return (
    <div className="flex min-h-dvh flex-col bg-bg text-ink">
      <a href="#conteudo" className="sr-only focus:not-sr-only focus:p-3">Ir para o conteúdo</a>
      <ServerWakingBanner />
      <header className="border-b border-slate-200 bg-white">
        <div className="mx-auto flex max-w-3xl items-center px-4 py-3">
          <Link to="/" className="min-h-11 content-center text-lg font-bold tracking-tight text-emerald-800 focus-visible:outline-3 focus-visible:outline-emerald-700">
            HumanoBill
          </Link>
        </div>
      </header>
      <main id="conteudo" className="mx-auto w-full max-w-3xl flex-1 px-4 py-8">
        <Outlet />
      </main>
      <footer className="border-t border-slate-200 px-4 py-6 text-center text-sm text-slate-600">
        Conteúdo informativo, não é orientação jurídica.{' '}
        <Link to="/privacidade" className="inline-block min-h-11 content-center underline">Privacidade</Link>
      </footer>
    </div>
  )
}
