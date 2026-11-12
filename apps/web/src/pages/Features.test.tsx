import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import Layout from '../components/Layout'
import { analysis, energyAnalysis } from '../test/fixtures'
import type { Analysis } from '../types/Analysis'
import Analyze from './Analyze'
import Landing from './Landing'
import Privacy from './Privacy'
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
            <Route path="analisar" element={<Analyze />} />
            <Route path="r/:id" element={<Result />} />
            <Route path="privacidade" element={<Privacy />} />
          </Route>
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  )
  return fetchMock
}

const byId = (list: Record<string, Analysis>): Handler => (url, init) => {
  const m = /\/analyses\/([^/?]+)$/.exec(url)
  return init?.method === 'GET' || !init?.method ? json(m ? list[m[1]] : {}) : json({})
}

beforeEach(() => vi.unstubAllGlobals())
afterEach(() => vi.unstubAllGlobals())

describe('RF05: corrigir dados', () => {
  it('alerta de soma divergente ao vivo e some quando a soma bate', async () => {
    mount(byId({ a1: analysis }), '/r/a1')
    await userEvent.click(await screen.findByRole('button', { name: 'Os dados estão errados? Corrigir' }))
    const dialog = screen.getByRole('dialog', { name: 'Conferir dados' })
    expect(dialog).toHaveAttribute('aria-modal', 'true')
    expect(within(dialog).queryByRole('alert')).toBeNull()

    const total = within(dialog).getByLabelText('Total da fatura')
    await userEvent.clear(total)
    await userEvent.type(total, '13000')
    expect(within(dialog).getByRole('alert')).toHaveTextContent('A soma dos itens não bate com o total. Confira os valores')

    await userEvent.clear(total)
    await userEvent.type(total, '12980')
    expect(within(dialog).queryByRole('alert')).toBeNull()
  })

  it('Esc fecha e devolve o foco ao link', async () => {
    mount(byId({ a1: analysis }), '/r/a1')
    const link = await screen.findByRole('button', { name: /Corrigir/ })
    await userEvent.click(link)
    await userEvent.keyboard('{Escape}')
    expect(screen.queryByRole('dialog')).toBeNull()
    expect(link).toHaveFocus()
  })

  it('salvar envia a fatura inteira no PATCH e atualiza o resultado, sem IA e rápido', async () => {
    const updated: Analysis = { ...analysis, bill: { ...analysis.bill, total_cents: 12980 }, findings: [{ ...analysis.findings[0], severity: 'indevido' }], recoverable_cents: 1290 }
    const fetchMock = mount((_url, init) => (init?.method === 'PATCH' ? json(updated) : json(analysis)), '/r/a1')
    await userEvent.click(await screen.findByRole('button', { name: /Corrigir/ }))
    const dialog = screen.getByRole('dialog')
    const value = within(dialog).getByLabelText('Valor', { selector: '#' + (within(dialog).getAllByLabelText('Valor')[0] as HTMLElement).id })
    await userEvent.clear(value)
    await userEvent.type(value, '11990')

    const t0 = performance.now()
    await userEvent.click(within(dialog).getByRole('button', { name: 'Salvar' }))
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull())
    expect(performance.now() - t0).toBeLessThan(1000)

    const call = fetchMock.mock.calls.find(([, i]) => i?.method === 'PATCH')!
    const body = JSON.parse(call[1]!.body as string)
    expect(Object.keys(body)).toEqual(['bill'])
    expect(body.bill.items[0].amount_cents).toBe(11990)
    expect(body.bill.kind).toBe('telecom')
    expect((await screen.findAllByText('Indevido')).length).toBeGreaterThan(0)
    // Só GET inicial e PATCH: nenhuma chamada de extração.
    expect(fetchMock.mock.calls.every(([u]) => !String(u).endsWith('/analyses'))).toBe(true)
  })

  it('energia: mostra consumo e tipo de leitura editáveis', async () => {
    mount(byId({ e1: energyAnalysis }), '/r/e1')
    await userEvent.click(await screen.findByRole('button', { name: /Corrigir/ }))
    expect(screen.getByLabelText('Consumo (kWh)')).toHaveValue('324')
    expect(screen.getByLabelText('Tipo de leitura')).toHaveValue('estimada')
  })

  it('mostra o erro da API ao salvar', async () => {
    mount((_u, init) => (init?.method === 'PATCH' ? json({ error: { code: 'invalid_correction', message: 'Dados inconsistentes.' } }, 400) : json(analysis)), '/r/a1')
    await userEvent.click(await screen.findByRole('button', { name: /Corrigir/ }))
    await userEvent.click(screen.getByRole('button', { name: 'Salvar' }))
    expect(await screen.findByText('Dados inconsistentes.')).toBeInTheDocument()
  })
})

describe('RF01: extração ao vivo do exemplo', () => {
  const live: Analysis = { ...analysis, id: 'live1', source: 'upload', bill: { ...analysis.bill, provider_name: 'Lida pela IA' } }
  const handler: Handler = (url, init) => {
    if (url.startsWith('/exemplos/')) return Promise.resolve(new Response(new Blob(['%PDF']), { status: 200 }))
    if (init?.method === 'POST' && url.endsWith('/analyses')) return json(live)
    return byId({ a1: analysis, live1: live })(url, init)
  }

  it('envia o PDF do exemplo por upload com consent e navega', async () => {
    const fetchMock = mount(handler, '/r/a1?exemplo=telecom-2')
    await userEvent.click(await screen.findByRole('button', { name: 'Analisar com IA de verdade' }))
    expect(await screen.findByText(/Lida pela IA/)).toBeInTheDocument()
    expect(fetchMock.mock.calls.some(([u]) => u === '/exemplos/telecom-2.pdf')).toBe(true)
    const post = fetchMock.mock.calls.find(([, i]) => i?.method === 'POST')!
    const form = post[1]!.body as FormData
    expect(form.get('consent')).toBe('true')
    expect((form.get('file') as File).name).toBe('telecom-2.pdf')
    expect(screen.queryByRole('button', { name: 'Analisar com IA de verdade' })).toBeNull()
  })

  it('mostra a mensagem do back e a sugestão de seguir com o exemplo', async () => {
    mount((u, i) => (i?.method === 'POST' && u.endsWith('/analyses') ? json({ error: { code: 'rate_limited', message: 'Muitas tentativas. Espere um pouco.' } }, 429) : handler(u, i)), '/r/a1')
    await userEvent.click(await screen.findByRole('button', { name: 'Analisar com IA de verdade' }))
    expect(await screen.findByRole('alert')).toHaveTextContent(/Muitas tentativas. Espere um pouco. Você pode continuar com o exemplo/)
  })

  it('não aparece em análises enviadas', async () => {
    mount(byId({ a1: { ...analysis, source: 'upload' } }), '/r/a1')
    await screen.findByText(/Vivaz Telecom/)
    expect(screen.queryByRole('button', { name: 'Analisar com IA de verdade' })).toBeNull()
  })
})

describe('Landing: exemplos agrupados', () => {
  it('lista telecom e energia e leva o exemplo na URL', async () => {
    mount((u) => (u.includes('/demo/') ? json({ ...analysis, id: 'a1' }) : json(analysis)), '/')
    await userEvent.click(screen.getByRole('button', { name: 'Testar com fatura de exemplo' }))
    const cel = screen.getByRole('group', { name: 'Celular/Internet' })
    const luz = screen.getByRole('group', { name: 'Luz' })
    expect(within(cel).getAllByRole('button')).toHaveLength(3)
    expect(within(luz).getByRole('button', { name: /Conta de luz alta e leitura estimada/ })).toBeInTheDocument()
    expect(within(luz).getByRole('button', { name: /Seguro embutido e multa/ })).toBeInTheDocument()
    expect(within(luz).getByRole('button', { name: /Bandeira cobrada errada/ })).toBeInTheDocument()
  })
})

describe('Energia: histórico e leitura', () => {
  it('gráfico acessível com resumo, tabela oculta, selo e bandeira', async () => {
    mount(byId({ e1: energyAnalysis }), '/r/e1')
    const chart = await screen.findByRole('img', { name: 'Consumo de outubro: 324 kWh, 62% acima da média de 200 kWh' })
    expect(chart).toBeInTheDocument()
    expect(screen.getByRole('table', { name: 'Consumo mensal em kWh' }).querySelectorAll('tbody tr')).toHaveLength(12)
    expect(screen.getByText(/Leitura estimada/)).toBeInTheDocument()
    expect(screen.getByText('Bandeira verde')).toBeInTheDocument()
  })
})

describe('RF06: explicar itens', () => {
  it('ICMS tem a frase de imposto estadual', async () => {
    mount(byId({ e1: energyAnalysis }), '/r/e1')
    const h = await screen.findByRole('heading', { name: 'ICMS 18%' })
    expect(within(h.closest('li')!).getByText('Imposto estadual cobrado sobre a energia.')).toBeInTheDocument()
  })

  it('todas as explicações têm até 25 palavras e sem juridiquês', async () => {
    mount(byId({ e1: energyAnalysis }), '/r/e1')
    await screen.findByRole('heading', { name: 'ICMS 18%' })
    for (const it of [...energyAnalysis.bill.items, ...analysis.bill.items]) {
      expect(it.plain_explanation.trim().split(/\s+/).length).toBeLessThanOrEqual(25)
      expect(it.plain_explanation).not.toMatch(/supramencionad|outrossim|ensejar/i)
    }
    const section = screen.getByRole('heading', { name: 'Sua conta explicada' }).closest('section')!
    for (const p of section.querySelectorAll('li p.text-muted')) {
      expect(p.textContent!.trim().split(/\s+/).length).toBeLessThanOrEqual(25)
      expect(p.textContent).not.toMatch(/supramencionad|outrossim|ensejar/i)
    }
  })
})

describe('RF15: privacidade', () => {
  it('aviso de consentimento: "Saiba mais" leva às 4 seções', async () => {
    mount(() => json({}), '/analisar')
    await userEvent.click(screen.getByRole('link', { name: 'Saiba mais' }))
    for (const t of ['O que processamos', 'Quem processa', 'Por quanto tempo', 'O que nunca guardamos'])
      expect(await screen.findByRole('heading', { name: t })).toBeInTheDocument()
  })

  it('rodapé: Privacidade em qualquer página, com data de atualização', async () => {
    mount(() => json({}), '/')
    await userEvent.click(screen.getByRole('link', { name: 'Privacidade' }))
    expect(await screen.findByText('11/11/2026')).toBeInTheDocument()
    expect(screen.getByText(/Atualizada em/)).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'Privacidade' })).toBeInTheDocument()
  })
})
