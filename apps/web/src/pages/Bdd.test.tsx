import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import Layout from '../components/Layout'
import { analysis } from '../test/fixtures'
import Landing from './Landing'
import Result from './Result'

const json = (body: unknown, status = 200) =>
  Promise.resolve(new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } }))

type Handler = (url: string, init?: RequestInit) => Promise<Response>

function mount(handler: Handler, path: string) {
  const fetchMock = vi.fn(handler)
  vi.stubGlobal('fetch', fetchMock)
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={[path]}>
        <Routes>
          <Route element={<Layout />}>
            <Route index element={<Landing />} />
            <Route path="r/:id" element={<Result />} />
          </Route>
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  )
  return fetchMock
}

beforeEach(() => vi.unstubAllGlobals())
afterEach(() => vi.unstubAllGlobals())

describe('RF13/RF14: link inexistente e expirado', () => {
  it('404 mostra "Resultado não encontrado"', async () => {
    mount(() => json({ error: { code: 'not_found', message: 'Resultado não encontrado' } }, 404), '/r/x')
    expect(await screen.findByRole('heading', { name: 'Resultado não encontrado' })).toBeInTheDocument()
  })

  it('410 mostra expirou e link para analisar de novo', async () => {
    mount(() => json({ error: { code: 'expired', message: 'Expirou' } }, 410), '/r/x')
    expect(await screen.findByRole('heading', { name: 'Este resultado expirou. Analise de novo.' })).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'Analisar de novo' })).toHaveAttribute('href', '/analisar')
  })
})

describe('RF13: Copiar link', () => {
  it('copia a URL atual e avisa "Link copiado"', async () => {
    const writeText = vi.fn(() => Promise.resolve())
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true })
    mount(() => json(analysis), '/r/a1')
    await userEvent.click(await screen.findByRole('button', { name: /Copiar link/ }))
    expect(writeText).toHaveBeenCalledWith(window.location.href)
    expect(await screen.findByText('Link copiado')).toBeInTheDocument()
  })
})

describe('RF01: exemplo de telecom', () => {
  it('escolhe o celular, faz POST e abre o resultado', async () => {
    const fetchMock = mount(() => json({ ...analysis, id: 'd1' }), '/')
    await userEvent.click(screen.getByRole('button', { name: 'Testar com fatura de exemplo' }))
    await userEvent.click(screen.getByRole('button', { name: /Celular com serviços extras/ }))
    expect(within(await screen.findByRole("article")).getByText("Suspeito — confira")).toBeInTheDocument()
    const call = fetchMock.mock.calls.find(([, i]) => i?.method === 'POST')!
    expect(call[0]).toMatch(/\/analyses\/demo\/telecom-1$/)
  })
})
