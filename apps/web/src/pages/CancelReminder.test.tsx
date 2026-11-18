import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import CancelReminder from './CancelReminder'

function json(body: unknown, status = 200) {
  return Promise.resolve(new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } }))
}

function setup(reply: () => Promise<Response>) {
  const fetchMock = vi.fn((_url: string, _init?: RequestInit) => reply())
  vi.stubGlobal('fetch', fetchMock)
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={['/lembrete/cancelar/tok123']}>
        <Routes>
          <Route path="lembrete/cancelar/:token" element={<CancelReminder />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  )
  return fetchMock
}

beforeEach(() => vi.unstubAllGlobals())
afterEach(() => vi.unstubAllGlobals())

describe('CancelReminder: Cancelar lembrete (RF19)', () => {
  // Cenário "Cancelamento pelo link do e-mail"
  it('ao abrir o link, cancela uma única vez e confirma que o e-mail foi apagado', async () => {
    const f = setup(() => json({ message: 'Lembrete cancelado e e-mail apagado' }))
    expect(await screen.findByText('Lembrete cancelado e e-mail apagado')).toBeInTheDocument()
    expect(f).toHaveBeenCalledTimes(1)
    expect(String(f.mock.calls[0][0])).toMatch(/\/reminders\/tok123\/cancel$/)
  })

  // Cenário "Token inválido"
  it('token inexistente (ou já cancelado) mostra a mesma mensagem', async () => {
    setup(() => json({ error: { code: 'not_found', message: 'Este lembrete não existe ou já foi cancelado' } }, 404))
    expect(await screen.findByText('Este lembrete não existe ou já foi cancelado')).toBeInTheDocument()
  })

  it('erro de rede não diz que o lembrete foi cancelado', async () => {
    setup(() => Promise.reject(new TypeError('falhou')))
    expect(await screen.findByText(/Não consegui falar com o servidor/)).toBeInTheDocument()
    expect(screen.queryByText('Lembrete cancelado e e-mail apagado')).not.toBeInTheDocument()
  })
})
