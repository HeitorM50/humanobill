import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Stats } from '../types/Stats'
import Layout from '../components/Layout'
import Landing from './Landing'
import Panel from './Panel'

const json = (body: unknown, status = 200) =>
  Promise.resolve(new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } }))

const vazio: Stats = {
  min_group: 5,
  kinds: [
    { kind: 'telecom', total: null, has_hidden_findings: false },
    { kind: 'energy', total: null, has_hidden_findings: false },
  ],
  rules: [],
}

const comDados: Stats = {
  min_group: 5,
  kinds: [
    { kind: 'telecom', total: 20, has_hidden_findings: true },
    { kind: 'energy', total: null, has_hidden_findings: true },
  ],
  rules: [
    { rule_id: 'TEL-01', title: 'Serviço extra que você não pediu', kind: 'telecom', severity: 'suspeito', count: 12, percent: 60 },
    { rule_id: 'TEL-03', title: 'Multa de atraso acima de 2%', kind: 'telecom', severity: 'indevido', count: 5, percent: 25 },
  ],
}

function mount(handler: (url: string) => Promise<Response>, path = '/painel') {
  const fetchMock = vi.fn((url: string) => handler(url))
  vi.stubGlobal('fetch', fetchMock)
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={[path]}>
        <Routes>
          <Route element={<Layout />}>
            <Route index element={<Landing />} />
            <Route path="painel" element={<Panel />} />
          </Route>
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  )
  return fetchMock
}

beforeEach(() => vi.unstubAllGlobals())
afterEach(() => vi.unstubAllGlobals())

describe('#62: painel coletivo', () => {
  it('mostra título, aviso de anonimato e estado vazio amigável', async () => {
    const fetchMock = mount(() => json(vazio))
    expect(screen.getByRole('heading', { level: 1, name: 'O que o HumanoBill já encontrou' })).toBeInTheDocument()
    expect(
      screen.getByText('Dados anônimos das faturas enviadas na demo pública. Grupos com menos de 5 casos não aparecem.'),
    ).toBeInTheDocument()
    expect(await screen.findByText('Ainda estamos juntando os primeiros casos')).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'Enviar minha fatura' })).toHaveAttribute('href', '/analisar')
    expect(screen.queryByRole('list')).not.toBeInTheDocument()
    expect(fetchMock.mock.calls[0][0]).toMatch(/\/stats$/)
  })

  it('barras com texto e número, não só cor; grupo pequeno vira "poucos casos ainda"', async () => {
    mount(() => json(comDados))
    const telecom = await screen.findByRole('region', { name: 'Celular, internet e TV' })
    expect(within(telecom).getByText('20 faturas enviadas')).toBeInTheDocument()

    const itens = within(telecom).getAllByRole('listitem')
    expect(itens).toHaveLength(2)
    expect(within(itens[0]).getByRole('heading', { name: 'Serviço extra que você não pediu' })).toBeInTheDocument()
    expect(itens[0]).toHaveTextContent('60% das faturas · 12 análises')
    expect(itens[0]).toHaveTextContent('Suspeito')
    expect(itens[1]).toHaveTextContent('25% das faturas · 5 análises')
    expect(itens[1]).toHaveTextContent('Indevido')
    expect(within(telecom).getByText('Outros achados têm poucos casos ainda e não aparecem.')).toBeInTheDocument()

    // A barra é só reforço visual: fica fora da leitura de tela.
    const barra = itens[0].querySelector('[aria-hidden="true"] > div') as HTMLElement
    expect(barra.style.width).toBe('60%')

    const luz = screen.getByRole('region', { name: 'Conta de luz' })
    expect(within(luz).getAllByText(/Poucos casos ainda/).length).toBeGreaterThan(0)
    expect(within(luz).queryByRole('listitem')).not.toBeInTheDocument()
  })

  it('erro do servidor mostra mensagem e permite tentar de novo', async () => {
    let chamadas = 0
    mount(() => (++chamadas === 1 ? json({ error: { code: 'internal', message: 'x' } }, 500) : json(comDados)))
    expect(await screen.findByRole('alert')).toHaveTextContent('Não consegui carregar os números.')
    await userEvent.click(screen.getByRole('button', { name: 'Tentar de novo' }))
    expect(await screen.findByRole('region', { name: 'Celular, internet e TV' })).toBeInTheDocument()
  })

  it('a página inicial e o rodapé levam ao painel', async () => {
    mount(() => json(vazio), '/')
    const links = screen.getAllByRole('link', { name: /o que (o HumanoBill )?já encontr/i })
    expect(links.length).toBeGreaterThanOrEqual(2)
    for (const l of links) expect(l).toHaveAttribute('href', '/painel')
    await userEvent.click(links[0])
    expect(await screen.findByRole('heading', { level: 1, name: 'O que o HumanoBill já encontrou' })).toBeInTheDocument()
  })
})
