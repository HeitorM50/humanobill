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

  it('ENE-07 usa "Autorizei"/"Não autorizei" e envia no', async () => {
    const q = { id: 'ENE-07:i6', rule_id: 'ENE-07', item_id: 'i6', text: 'Você autorizou Seguro Residencial Luzia na sua conta de luz?', kind: 'yes_no' as const }
    const base: Analysis = { ...analysis, questions: [q] }
    const fetchMock = setup(base, () => json({ ...base, answers: { 'ENE-07:i6': { type: 'no' } } }))
    expect(await screen.findByRole('button', { name: 'Autorizei' })).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Não autorizei' }))
    await waitFor(() => expect(screen.getByRole('button', { name: 'Não autorizei' })).toHaveAttribute('aria-pressed', 'true'))
    const call = fetchMock.mock.calls.find(([, i]) => i?.method === 'PATCH')!
    expect(JSON.parse(call[1]!.body as string)).toEqual({ answers: { 'ENE-07:i6': { type: 'no' } } })
  })

  it('outras regras (ENE-04) usam Sim/Não', async () => {
    const q = { id: 'ENE-04:ciclo', rule_id: 'ENE-04', item_id: null, text: 'É sua primeira conta?', kind: 'yes_no' as const }
    const fetchMock = setup({ ...analysis, questions: [q] }, () => json(analysis))
    const group = within(await screen.findByRole('group', { name: q.text }))
    expect(group.getByRole('button', { name: 'Sim' })).toBeInTheDocument()
    await userEvent.click(group.getByRole('button', { name: 'Não' }))
    await waitFor(() => expect(fetchMock.mock.calls.some(([, i]) => i?.method === 'PATCH')).toBe(true))
    const call = fetchMock.mock.calls.find(([, i]) => i?.method === 'PATCH')!
    expect(JSON.parse(call[1]!.body as string)).toEqual({ answers: { 'ENE-04:ciclo': { type: 'no' } } })
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

describe('Result: valor recuperável (RF10)', () => {
  const undue = { ...analysis.findings[0], severity: 'indevido' as const, amount_cents: 2280 }

  it('não pago: "Até R$ 22,80 podem ser retirados da fatura"', async () => {
    setup({ ...analysis, findings: [undue], paid: false, recoverable_cents: 2280 }, () => json(analysis))
    expect(await screen.findByText('Até R$ 22,80 podem ser retirados da fatura')).toBeInTheDocument()
  })

  it('pago: mostra o dobro e a ressalva', async () => {
    setup({ ...analysis, findings: [undue], paid: true, recoverable_cents: 4560 }, () => json(analysis))
    expect(await screen.findByText('Até R$ 45,60 recuperáveis (devolução em dobro, CDC art. 42)')).toBeInTheDocument()
    expect(screen.getByText('Estimativa. O valor final depende da empresa ou do órgão')).toBeInTheDocument()
  })

  it('não informado: valor simples e convite para a pergunta', async () => {
    setup({ ...analysis, findings: [undue], paid: null, recoverable_cents: 2280 }, () => json(analysis))
    expect(await screen.findByText('R$ 22,80 parecem indevidos')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Responder' }))
    expect(screen.getByRole('button', { name: 'Sim' })).toHaveFocus()
  })

  it('sem indevidos: sem destaque; com suspeito mostra "para conferir"', async () => {
    setup(analysis, () => json(analysis))
    expect(await screen.findByText(/R\$\s12,90 para conferir/)).toBeInTheDocument()
    expect(screen.queryByText(/recuperáveis|retirados da fatura|parecem indevidos/)).toBeNull()
  })

  it('sem indevidos nem suspeitos: nenhum cartão', async () => {
    setup({ ...analysis, findings: [] }, () => json(analysis))
    await screen.findByText(/Tudo certo com esta conta/)
    expect(screen.queryByText(/para conferir/)).toBeNull()
  })

  it('atualiza o valor ao responder "Não contratei"', async () => {
    const updated: Analysis = { ...analysis, findings: [{ ...undue, amount_cents: 1290 }], recoverable_cents: 1290, paid: false }
    setup({ ...analysis, paid: false }, () => json(updated))
    await userEvent.click(await screen.findByRole('button', { name: 'Não contratei' }))
    expect(await screen.findByText('Até R$ 12,90 podem ser retirados da fatura')).toBeInTheDocument()
  })
})

describe('Result: contestação (RF11/RF12)', () => {
  const undue = { ...analysis.findings[0], id: 'TEL-01:i1', severity: 'indevido' as const, amount_cents: 2280, item_refs: ['i1'] }
  const suspect = analysis.findings[0]
  const base: Analysis = { ...analysis, findings: [undue, suspect], paid: false, recoverable_cents: 2280 }
  const dispute = {
    text: 'Prezados,\n\nContesto o item CLUBE VZ MUS, R$ 22,80, outubro de 2026 (CDC art. 39). Peço a devolução. Qual o protocolo?',
    total_cents: 2280,
    steps: [
      { title: 'SAC da operadora', description: 'Ligue e anote o protocolo.', url: null },
      { title: 'Anatel', description: 'Depois de 30 dias sem resposta.', url: 'https://www.anatel.gov.br' },
      { title: 'consumidor.gov.br', description: 'Registre a reclamação.', url: 'https://www.consumidor.gov.br' },
    ],
    deadlines: ['A operadora tem 30 dias para responder', 'Você pode contestar cobranças de até 3 anos'],
    disclaimer: 'Conteúdo informativo, não é orientação jurídica',
  }

  function setupDispute() {
    const fetchMock = vi.fn((url: string, init?: RequestInit) =>
      init?.method === 'POST' && String(url).endsWith('/dispute') ? json(dispute) : json(base),
    )
    vi.stubGlobal('fetch', fetchMock)
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    render(
      <QueryClientProvider client={qc}>
        <MemoryRouter initialEntries={['/r/a1']}>
          <Routes><Route path="r/:id" element={<Result />} /></Routes>
        </MemoryRouter>
      </QueryClientProvider>,
    )
    return fetchMock
  }

  it('indevido vem marcado, suspeito desmarcado, e o resumo conta só os marcados', async () => {
    setupDispute()
    const boxes = await screen.findAllByRole('checkbox', { name: 'Incluir na contestação' })
    expect(boxes).toHaveLength(2)
    expect(boxes[0]).toBeChecked()
    expect(boxes[1]).not.toBeChecked()
    expect(screen.getByText(/1 problema selecionado · R\$\s22,80/)).toBeInTheDocument()
  })

  it('sem seleção o botão fica desabilitado', async () => {
    setupDispute()
    const boxes = await screen.findAllByRole('checkbox')
    await userEvent.click(boxes[0])
    expect(screen.getByRole('button', { name: 'Gerar contestação' })).toBeDisabled()
  })

  it('gera o texto, mostra o aviso, copia e avisa "Copiado!"', async () => {
    const writeText = vi.fn(() => Promise.resolve())
    const fetchMock = setupDispute()
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true })
    await userEvent.click(await screen.findByRole('button', { name: 'Gerar contestação' }))
    expect(await screen.findByText(/Contesto o item CLUBE VZ MUS, R\$ 22,80/)).toBeInTheDocument()
    expect(screen.getAllByText('Conteúdo informativo, não é orientação jurídica').length).toBeGreaterThan(0)
    const call = fetchMock.mock.calls.find(([, i]) => i?.method === 'POST')!
    expect(JSON.parse(call[1]!.body as string)).toEqual({ finding_ids: ['TEL-01:i1'] })

    await userEvent.click(screen.getByRole('button', { name: 'Copiar' }))
    expect(writeText).toHaveBeenCalledWith(dispute.text)
    expect((await screen.findAllByText(/Copiado!/)).length).toBeGreaterThan(0)
  })

  it('mostra passos na ordem e prazos; muda a seleção e avisa que está desatualizado', async () => {
    setupDispute()
    await userEvent.click(await screen.findByRole('button', { name: 'Gerar contestação' }))
    const steps = await screen.findAllByRole('listitem')
    const titles = ['SAC da operadora', 'Anatel', 'consumidor.gov.br']
    const order = titles.map((t) => steps.findIndex((li) => li.textContent?.includes(t) && li.querySelector('span')))
    expect(order.every((i) => i >= 0)).toBe(true)
    expect([...order].sort((a, b) => a - b)).toEqual(order)
    expect(screen.getByText('A operadora tem 30 dias para responder')).toBeInTheDocument()
    expect(screen.getByText('Você pode contestar cobranças de até 3 anos')).toBeInTheDocument()
    const link = screen.getByRole('link', { name: 'Anatel (abre em nova aba)' })
    expect(link).toHaveAttribute('target', '_blank')

    await userEvent.click(screen.getAllByRole('checkbox')[1])
    expect(screen.getByText(/texto está desatualizado/)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Gerar de novo' })).toBeEnabled()
  })
})

describe('Result: rodapé', () => {
  it('oferece comparar com outra fatura, já com esta análise no primeiro campo', async () => {
    setup(analysis, () => json(analysis))
    const link = await screen.findByRole('link', { name: 'Comparar com outra fatura' })
    expect(link).toHaveAttribute('href', `/comparar?a=${analysis.id}`)
    expect(link).toHaveClass('min-h-11.5')
  })

  it('codifica o código da análise no link de comparar', async () => {
    setup({ ...analysis, id: 'a b&c' }, () => json(analysis))
    const link = await screen.findByRole('link', { name: 'Comparar com outra fatura' })
    expect(link).toHaveAttribute('href', '/comparar?a=a%20b%26c')
  })
})
