import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { analysis } from '../test/fixtures'
import type { Analysis } from '../types/Analysis'
import Result from './Result'

function json(body: unknown, status = 200) {
  return Promise.resolve(new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } }))
}

function setup(initial: Analysis, onPatch: (body: unknown) => Promise<Response>) {
  const fetchMock = vi.fn((_url: string, init?: RequestInit) =>
    init?.method === 'PATCH' ? onPatch(JSON.parse(init.body as string)) : json(initial),
  )
  vi.stubGlobal('fetch', fetchMock)
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={['/r/a1']}>
        <Routes>
          <Route path="r/:id" element={<Result />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  )
  return fetchMock
}

beforeEach(() => vi.unstubAllGlobals())
afterEach(() => vi.unstubAllGlobals())

describe('Result: perguntas rápidas', () => {
  it('envia PATCH com o mapa completo e o achado vira Indevido', async () => {
    const withPrior: Analysis = { ...analysis, answers: { 'TEL-05:plano': { type: 'amount', value: 9990 } } }
    const updated: Analysis = {
      ...withPrior,
      answers: { ...withPrior.answers, 'TEL-01:i3': { type: 'no' } },
      findings: [{ ...analysis.findings[0], severity: 'indevido' }],
      recoverable_cents: 1290,
    }
    const fetchMock = setup(withPrior, () => json(updated))

    const card = await screen.findByRole('article')
    expect(within(card).getByText('Suspeito — confira')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Não contratei' }))

    await waitFor(() => expect(within(screen.getByRole('article')).getByText('Indevido')).toBeInTheDocument())
    const call = fetchMock.mock.calls.find(([, i]) => i?.method === 'PATCH')!
    expect(call[0]).toMatch(/\/analyses\/a1$/)
    expect(JSON.parse(call[1]!.body as string)).toEqual({
      answers: { 'TEL-05:plano': { type: 'amount', value: 9990 }, 'TEL-01:i3': { type: 'no' } },
    })
    expect(screen.getByRole('button', { name: 'Não contratei' })).toHaveAttribute('aria-pressed', 'true')
  })

  it('mostra "Salvando…" e a mensagem de erro do back-end', async () => {
    setup(analysis, () => json({ error: { code: 'expired', message: 'Este resultado expirou.' } }, 410))
    await userEvent.click(await screen.findByRole('button', { name: 'Contratei' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Este resultado expirou.')
  })

  it('"Já pagou" envia paid', async () => {
    const fetchMock = setup(analysis, () => json({ ...analysis, paid: true }))
    await userEvent.click(await screen.findByRole('button', { name: 'Sim' }))
    await waitFor(() => expect(screen.getByRole('button', { name: 'Sim' })).toHaveAttribute('aria-pressed', 'true'))
    const call = fetchMock.mock.calls.find(([, i]) => i?.method === 'PATCH')!
    expect(JSON.parse(call[1]!.body as string)).toEqual({ paid: true })
  })
})

describe('Result: sem problemas', () => {
  it('mostra "Tudo certo com esta conta" e avisa das perguntas pendentes', async () => {
    setup({ ...analysis, findings: [] }, () => json(analysis))
    expect(await screen.findByText(/Tudo certo com esta conta/)).toBeInTheDocument()
    expect(screen.getByText(/Responder pode mudar este resultado/)).toBeInTheDocument()
  })

  it('sem perguntas pendentes não faz o aviso', async () => {
    setup({ ...analysis, findings: [], questions: [] }, () => json(analysis))
    expect(await screen.findByText(/Tudo certo com esta conta/)).toBeInTheDocument()
    expect(screen.queryByText(/Responder pode mudar/)).toBeNull()
  })
})
