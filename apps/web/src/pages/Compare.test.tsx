import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { ComparedItem } from '../types/ComparedItem'
import type { Comparison } from '../types/Comparison'
import App from '../App'
import Compare from './Compare'
import Landing from './Landing'

const ID_OUT = '5b0e8f0e-3b8a-4f5e-9c1a-0a1b2c3d4e5f'
const ID_NOV = '9d4c1a77-52a3-4e0b-8f6d-7e8f9a0b1c2d'

const json = (body: unknown, status = 200) =>
  Promise.resolve(new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } }))

const erro = (status: number, code: string, message: string) => json({ error: { code, message } }, status)

function item(over: Partial<ComparedItem>): ComparedItem {
  return {
    description: 'Plano Vivaz Controle 20GB',
    category: 'plano',
    before_cents: 9990,
    after_cents: 9990,
    delta_cents: 0,
    delta_pct: 0,
    new_charge: false,
    ...over,
  }
}

const seguroTela = item({
  description: 'Seguro Tela',
  category: 'servico_adicional',
  before_cents: null,
  after_cents: 990,
  delta_cents: 990,
  delta_pct: null,
  new_charge: true,
})

const base: Comparison = {
  kind: 'telecom',
  provider_name: 'Vivaz Telecom',
  before_month: '2026-10',
  after_month: '2026-11',
  before_total_cents: 9990,
  after_total_cents: 10980,
  total_delta_cents: 990,
  total_delta_pct: 9.9,
  added: [seguroTela],
  removed: [],
  changed: [],
  unchanged: [item({})],
}

function mount(handler: (url: string, init?: RequestInit) => Promise<Response>, path = '/comparar') {
  const fetchMock = vi.fn(handler)
  vi.stubGlobal('fetch', fetchMock)
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } })
  render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={[path]}>
        <Routes>
          <Route path="comparar" element={<Compare />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  )
  return fetchMock
}

async function preencherEComparar(a: string, b: string) {
  // `type` não aceita texto vazio: campo em branco é simplesmente não digitar.
  if (a) await userEvent.type(screen.getByLabelText('Primeira fatura'), a)
  if (b) await userEvent.type(screen.getByLabelText('Segunda fatura'), b)
  await userEvent.click(screen.getByRole('button', { name: 'Comparar' }))
}

const postBodies = (fetchMock: ReturnType<typeof vi.fn>) =>
  fetchMock.mock.calls.filter(([, init]) => init?.method === 'POST').map(([, init]) => JSON.parse(init.body as string))

beforeEach(() => vi.unstubAllGlobals())
afterEach(() => vi.unstubAllGlobals())

describe('RF16: comparar faturas', () => {
  // Cenário: Item novo aparece
  it('item novo aparece como "novo este mês", com a variação em reais e em %', async () => {
    const fetchMock = mount(() => json(base))
    await preencherEComparar(`https://humanobill.app/r/${ID_OUT}?exemplo=telecom-1`, ID_NOV.toUpperCase())

    const novos = await screen.findByRole('region', { name: 'Itens novos' })
    expect(within(novos).getByText('Seguro Tela')).toBeInTheDocument()
    expect(within(novos).getByText('Novo este mês')).toBeInTheDocument()
    expect(within(novos).getByText('+R$ 9,90')).toBeInTheDocument()

    const total = screen.getByRole('region', { name: 'Variação do total' })
    expect(total).toHaveTextContent('Sua conta subiu')
    expect(total).toHaveTextContent('R$ 9,90')
    expect(total).toHaveTextContent('9,9%')
    expect(total).toHaveTextContent('Vivaz Telecom')
    expect(total).toHaveTextContent('outubro de 2026')
    expect(total).toHaveTextContent('novembro de 2026')

    // Mandou só os ids (link e código crus viram uuid em minúsculas), pela rota certa.
    expect(fetchMock).toHaveBeenCalledTimes(1)
    expect(fetchMock.mock.calls[0][0]).toMatch(/\/comparisons$/)
    expect(postBodies(fetchMock)).toEqual([{ analysis_ids: [ID_OUT, ID_NOV] }])
  })

  it('serviço que virou cobrança nova ganha o marca-texto e a pergunta "você pediu isso?"', async () => {
    mount(() => json(base))
    await preencherEComparar(ID_OUT, ID_NOV)
    const novos = await screen.findByRole('region', { name: 'Itens novos' })
    expect(within(novos).getByText('Seguro Tela').closest('.marca-texto')).not.toBeNull()
    expect(within(novos).getByText(/Cobrança nova/)).toBeInTheDocument()
    expect(within(novos).getByText(/Você pediu isso\?/)).toBeInTheDocument()
  })

  it('item novo que não é serviço extra não ganha o marca-texto', async () => {
    const taxa = item({ description: 'Taxa de instalação', category: 'outros', before_cents: null, after_cents: 5000, delta_cents: 5000, delta_pct: null, new_charge: false })
    mount(() => json({ ...base, added: [taxa] }))
    await preencherEComparar(ID_OUT, ID_NOV)
    const novos = await screen.findByRole('region', { name: 'Itens novos' })
    expect(within(novos).getByText('Taxa de instalação').closest('.marca-texto')).toBeNull()
    expect(within(novos).queryByText(/Você pediu isso/)).toBeNull()
  })

  // Cenário: Operadoras diferentes
  it('operadoras diferentes mostram "Compare faturas da mesma operadora"', async () => {
    mount(() => erro(400, 'different_provider', 'Compare faturas da mesma operadora'))
    await preencherEComparar(ID_OUT, ID_NOV)
    expect(await screen.findByRole('alert')).toHaveTextContent('Compare faturas da mesma operadora')
    expect(screen.queryByRole('region', { name: 'Variação do total' })).toBeNull()
  })

  // Cenário: Mesma fatura duas vezes
  it('mesmo mês de referência mostra "Envie faturas de meses diferentes"', async () => {
    const fetchMock = mount(() => erro(400, 'same_month', 'Envie faturas de meses diferentes'))
    await preencherEComparar(ID_OUT, ID_OUT)
    expect(await screen.findByRole('alert')).toHaveTextContent('Envie faturas de meses diferentes')
    expect(postBodies(fetchMock)).toEqual([{ analysis_ids: [ID_OUT, ID_OUT] }])
  })

  it('resultado expirado ou inexistente mostra a mensagem do servidor', async () => {
    mount(() => erro(410, 'expired', 'Este resultado expirou. Analise de novo'))
    await preencherEComparar(ID_OUT, ID_NOV)
    expect(await screen.findByRole('alert')).toHaveTextContent('Este resultado expirou. Analise de novo')
  })

  it('sem conexão, diz que não conseguiu falar com o servidor', async () => {
    mount(() => Promise.reject(new TypeError('failed')))
    await preencherEComparar(ID_OUT, ID_NOV)
    expect(await screen.findByRole('alert')).toHaveTextContent('Não consegui falar com o servidor')
  })

  it('texto que não é link nem código mostra o erro no campo e não chama o servidor', async () => {
    const fetchMock = mount(() => json(base))
    await preencherEComparar('minha fatura', '')
    const primeiro = screen.getByLabelText('Primeira fatura')
    const segundo = screen.getByLabelText('Segunda fatura')
    expect(primeiro).toBeInvalid()
    expect(segundo).toBeInvalid()
    expect(screen.getAllByText(/Não achei um link de resultado/).length).toBeGreaterThan(0)
    expect(primeiro).toHaveFocus()
    expect(fetchMock).not.toHaveBeenCalled()
  })

  it('o erro do campo some quando a pessoa corrige e compara de novo', async () => {
    mount(() => json(base))
    await preencherEComparar('x', ID_NOV)
    expect(screen.getByLabelText('Primeira fatura')).toBeInvalid()
    await userEvent.clear(screen.getByLabelText('Primeira fatura'))
    await userEvent.type(screen.getByLabelText('Primeira fatura'), ID_OUT)
    await userEvent.click(screen.getByRole('button', { name: 'Comparar' }))
    await screen.findByRole('region', { name: 'Variação do total' })
    expect(screen.getByLabelText('Primeira fatura')).toBeValid()
  })

  it('preenche os campos com ?a= e ?b= do endereço', () => {
    mount(() => json(base), `/comparar?a=${ID_OUT}&b=${ID_NOV}`)
    expect(screen.getByLabelText('Primeira fatura')).toHaveValue(ID_OUT)
    expect(screen.getByLabelText('Segunda fatura')).toHaveValue(ID_NOV)
  })

  it('enquanto compara, o botão fica ocupado', async () => {
    let soltar: (r: Response) => void = () => {}
    mount(() => new Promise<Response>((r) => (soltar = r)))
    await preencherEComparar(ID_OUT, ID_NOV)
    const botao = await screen.findByRole('button', { name: /Comparando/ })
    expect(botao).toBeDisabled()
    expect(botao).toHaveAttribute('aria-busy', 'true')
    soltar(new Response(JSON.stringify(base), { status: 200 }))
    await waitFor(() => expect(screen.getByRole('button', { name: 'Comparar' })).toBeEnabled())
  })
})

describe('RF16: exemplo de comparação (P13)', () => {
  const ID_SET = '1a2b3c4d-1111-4222-8333-444455556666'
  const ID_OUT_DEMO = '7e8f9a0b-2222-4333-8444-555566667777'

  /** Roteia as chamadas: duas análises de demo e a comparação. */
  function demoHandler(over: { comparacao?: () => Promise<Response>; anterior?: () => Promise<Response>; atual?: () => Promise<Response> } = {}) {
    return (url: string) => {
      if (url.endsWith('/analyses/demo/telecom-1-anterior')) return over.anterior?.() ?? json({ id: ID_SET }, 201)
      if (url.endsWith('/analyses/demo/telecom-1')) return over.atual?.() ?? json({ id: ID_OUT_DEMO }, 201)
      if (url.endsWith('/comparisons')) return over.comparacao?.() ?? json(base)
      return json({}, 404)
    }
  }

  const botaoExemplo = () => screen.getByRole('button', { name: 'Ver um exemplo de comparação' })

  it('cria as duas análises de demo, compara e mostra o resultado', async () => {
    const fetchMock = mount(demoHandler())
    await userEvent.click(botaoExemplo())

    const novos = await screen.findByRole('region', { name: 'Itens novos' })
    expect(within(novos).getByText('Seguro Tela')).toBeInTheDocument()
    expect(within(novos).getByText('Novo este mês')).toBeInTheDocument()
    expect(screen.getByRole('region', { name: 'Variação do total' })).toHaveTextContent('Sua conta subiu')

    // Primeiro o mês anterior, depois o atual, e só então a comparação, com os dois ids.
    const chamadas = fetchMock.mock.calls.map(([url, init]) => `${init?.method ?? 'GET'} ${String(url).replace(/^.*(?=\/(analyses|comparisons))/, '')}`)
    expect(chamadas).toEqual([
      'POST /analyses/demo/telecom-1-anterior',
      'POST /analyses/demo/telecom-1',
      'POST /comparisons',
    ])
    expect(JSON.parse(fetchMock.mock.calls[2][1]?.body as string)).toEqual({ analysis_ids: [ID_SET, ID_OUT_DEMO] })
  })

  it('enquanto monta o exemplo, avisa e deixa os botões ocupados', async () => {
    let soltar: (r: Response) => void = () => {}
    mount(demoHandler({ anterior: () => new Promise<Response>((r) => (soltar = r)) }))
    await userEvent.click(botaoExemplo())

    expect(await screen.findByText('Montando o exemplo, só um instante')).toHaveAttribute('role', 'status')
    const ocupado = screen.getByRole('button', { name: /Montando o exemplo/ })
    expect(ocupado).toBeDisabled()
    expect(ocupado).toHaveAttribute('aria-busy', 'true')
    expect(screen.getByRole('button', { name: 'Comparar' })).toBeDisabled()

    soltar(new Response(JSON.stringify({ id: ID_SET }), { status: 201 }))
    await screen.findByRole('region', { name: 'Variação do total' })
    expect(screen.queryByText('Montando o exemplo, só um instante')).toBeNull()
    expect(botaoExemplo()).toBeEnabled()
  })

  it('se o servidor não responde, mostra o erro em português e não mostra comparação', async () => {
    mount(() => Promise.reject(new TypeError('failed')))
    await userEvent.click(botaoExemplo())
    expect(await screen.findByRole('alert')).toHaveTextContent('Não consegui falar com o servidor')
    expect(screen.queryByRole('region', { name: 'Variação do total' })).toBeNull()
    expect(botaoExemplo()).toBeEnabled()
  })

  it.each([
    [429, 'rate_limited', 'Muitas tentativas agora. Espere um pouco e tente de novo.'],
    [500, 'internal', 'Algo deu errado do nosso lado. Tente de novo.'],
  ])('se a segunda análise de demo falha com %i, mostra a mensagem do servidor e não compara', async (status, code, mensagem) => {
    const fetchMock = mount(demoHandler({ atual: () => erro(status, code, mensagem) }))
    await userEvent.click(botaoExemplo())

    expect(await screen.findByRole('alert')).toHaveTextContent(mensagem)
    expect(screen.queryByRole('region', { name: 'Variação do total' })).toBeNull()
    // Só as duas criações de demo: a comparação nem chega a ser pedida.
    expect(fetchMock).toHaveBeenCalledTimes(2)
    expect(fetchMock.mock.calls.every(([url]) => !String(url).endsWith('/comparisons'))).toBe(true)
    expect(botaoExemplo()).toBeEnabled()
  })

  it('enquanto o fluxo manual compara, o botão do exemplo fica desabilitado', async () => {
    let soltar: (r: Response) => void = () => {}
    const fetchMock = mount(() => new Promise<Response>((r) => (soltar = r)))
    await preencherEComparar(ID_OUT, ID_NOV)
    await screen.findByRole('button', { name: /Comparando/ })

    expect(botaoExemplo()).toBeDisabled()
    await userEvent.click(botaoExemplo())
    expect(fetchMock).toHaveBeenCalledTimes(1)

    soltar(new Response(JSON.stringify(base), { status: 200 }))
    await waitFor(() => expect(botaoExemplo()).toBeEnabled())
  })

  it('duplo clique no exemplo, com a primeira chamada pendente, faz um único pedido', async () => {
    let soltar: (r: Response) => void = () => {}
    const fetchMock = mount(demoHandler({ anterior: () => new Promise<Response>((r) => (soltar = r)) }))
    await userEvent.dblClick(botaoExemplo())

    const demos = () => fetchMock.mock.calls.filter(([url]) => String(url).endsWith('/analyses/demo/telecom-1-anterior'))
    expect(demos()).toHaveLength(1)
    expect(fetchMock).toHaveBeenCalledTimes(1)

    soltar(new Response(JSON.stringify({ id: ID_SET }), { status: 201 }))
    await screen.findByRole('region', { name: 'Variação do total' })
    expect(demos()).toHaveLength(1)
    expect(fetchMock).toHaveBeenCalledTimes(3)
  })

  it('se a comparação falha, mostra a mensagem do servidor e deixa tentar de novo', async () => {
    let tentativa = 0
    mount(demoHandler({ comparacao: () => (tentativa++ === 0 ? erro(500, 'internal', 'Algo deu errado do nosso lado. Tente de novo.') : json(base)) }))
    await userEvent.click(botaoExemplo())
    expect(await screen.findByRole('alert')).toHaveTextContent('Algo deu errado do nosso lado')

    await userEvent.click(botaoExemplo())
    await screen.findByRole('region', { name: 'Variação do total' })
    expect(screen.queryByRole('alert')).toBeNull()
  })

  it('o fluxo de colar dois links continua igual e não usa as análises de demo', async () => {
    const fetchMock = mount(demoHandler())
    await preencherEComparar(ID_OUT, ID_NOV)
    await screen.findByRole('region', { name: 'Variação do total' })
    expect(fetchMock).toHaveBeenCalledTimes(1)
    expect(fetchMock.mock.calls[0][0]).toMatch(/\/comparisons$/)
    expect(postBodies(fetchMock)).toEqual([{ analysis_ids: [ID_OUT, ID_NOV] }])
  })
})

describe('RF16: leitura da comparação', () => {
  it('valor que subiu, caiu, item que saiu e itens sem mudança', async () => {
    const subiu = item({ description: 'Plano Vivaz Controle 20GB', before_cents: 9990, after_cents: 10990, delta_cents: 1000, delta_pct: 10 })
    const caiu = item({ description: 'Internet extra', category: 'outros', before_cents: 2000, after_cents: 1500, delta_cents: -500, delta_pct: -25 })
    const saiu = item({ description: 'Clube Vivaz Música', category: 'servico_adicional', before_cents: 1290, after_cents: null, delta_cents: -1290, delta_pct: null })
    const igual = item({ description: 'Parcela do aparelho 4/12', category: 'outros', before_cents: 5000, after_cents: 5000 })
    mount(() =>
      json({
        ...base,
        after_total_cents: 8700,
        total_delta_cents: -1290,
        total_delta_pct: -12.9,
        added: [],
        changed: [subiu, caiu],
        removed: [saiu],
        unchanged: [igual],
      }),
    )
    await preencherEComparar(ID_OUT, ID_NOV)

    const total = await screen.findByRole('region', { name: 'Variação do total' })
    expect(total).toHaveTextContent('Sua conta caiu')
    expect(total).toHaveTextContent('R$ 12,90')
    expect(total).toHaveTextContent('12,9%')
    expect(screen.queryByRole('region', { name: 'Itens novos' })).toBeNull()

    const mudou = screen.getByRole('region', { name: 'Mudou de valor' })
    expect(within(mudou).getByText('+R$ 10,00')).toBeInTheDocument()
    expect(within(mudou).getByText(/Era R\$ 99,90 e agora é R\$ 109,90 \(10,0% a mais\)/)).toBeInTheDocument()
    expect(within(mudou).getByText('−R$ 5,00')).toBeInTheDocument()
    expect(within(mudou).getByText(/Era R\$ 20,00 e agora é R\$ 15,00 \(25,0% a menos\)/)).toBeInTheDocument()

    const saiuSec = screen.getByRole('region', { name: 'Saiu da conta' })
    expect(within(saiuSec).getByText('Clube Vivaz Música')).toBeInTheDocument()
    expect(within(saiuSec).getByText('−R$ 12,90')).toBeInTheDocument()

    const sem = screen.getByText(/Sem mudança \(1 item\)/)
    expect(sem.closest('details')).not.toHaveAttribute('open')
    expect(screen.getByText('Parcela do aparelho 4/12')).toBeInTheDocument()
  })

  it('serviço que era grátis e passou a cobrar é cobrança nova, sem porcentagem', async () => {
    const clube = item({ description: 'Clube Vivaz Música', category: 'servico_adicional', before_cents: 0, after_cents: 1290, delta_cents: 1290, delta_pct: null, new_charge: true })
    mount(() => json({ ...base, added: [], changed: [clube], unchanged: [] }))
    await preencherEComparar(ID_OUT, ID_NOV)
    const mudou = await screen.findByRole('region', { name: 'Mudou de valor' })
    expect(within(mudou).getByText(/Era R\$ 0,00 e agora é R\$ 12,90/)).toBeInTheDocument()
    expect(within(mudou).queryByText(/% a mais/)).toBeNull()
    expect(within(mudou).getByText(/Cobrança nova/)).toBeInTheDocument()
  })

  it('totais iguais e itens iguais: diz que não mudou nada', async () => {
    mount(() =>
      json({ ...base, after_total_cents: 9990, total_delta_cents: 0, total_delta_pct: 0, added: [], changed: [], removed: [] }),
    )
    await preencherEComparar(ID_OUT, ID_NOV)
    const total = await screen.findByRole('region', { name: 'Variação do total' })
    expect(total).toHaveTextContent('Sua conta não mudou')
    expect(screen.getByText('Os itens são os mesmos nos dois meses.')).toBeInTheDocument()
  })

  it('total anterior zero: mostra os reais e não inventa porcentagem', async () => {
    mount(() => json({ ...base, before_total_cents: 0, after_total_cents: 990, total_delta_pct: null }))
    await preencherEComparar(ID_OUT, ID_NOV)
    const total = await screen.findByRole('region', { name: 'Variação do total' })
    expect(total).toHaveTextContent('R$ 9,90')
    expect(total).not.toHaveTextContent('%')
  })

  it('os dois meses aparecem na ordem em que o servidor devolveu (mais antigo primeiro)', async () => {
    mount(() => json(base))
    await preencherEComparar(ID_NOV, ID_OUT)
    const total = await screen.findByRole('region', { name: 'Variação do total' })
    const texto = total.textContent ?? ''
    expect(texto.indexOf('outubro de 2026')).toBeLessThan(texto.indexOf('novembro de 2026'))
  })

  it('tem um único h1 e os títulos de seção são h2', async () => {
    mount(() => json(base))
    await preencherEComparar(ID_OUT, ID_NOV)
    await screen.findByRole('region', { name: 'Variação do total' })
    expect(screen.getAllByRole('heading', { level: 1 })).toHaveLength(1)
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent('Comparar duas faturas')
    expect(screen.getByRole('heading', { name: 'Itens novos', level: 2 })).toBeInTheDocument()
  })
})

describe('RF16: como chegar na comparação', () => {
  it('a página inicial tem um link discreto para comparar faturas', () => {
    vi.stubGlobal('fetch', vi.fn(() => json({ status: 'ok' })))
    const qc = new QueryClient()
    render(
      <QueryClientProvider client={qc}>
        <MemoryRouter>
          <Landing />
        </MemoryRouter>
      </QueryClientProvider>,
    )
    expect(screen.getByRole('link', { name: /comparar duas faturas/i })).toHaveAttribute('href', '/comparar')
  })

  it('a rota /comparar abre a página', async () => {
    vi.stubGlobal('fetch', vi.fn(() => json({ status: 'ok' })))
    window.history.pushState({}, '', '/comparar')
    render(<App />)
    expect(await screen.findByRole('heading', { level: 1, name: 'Comparar duas faturas' })).toBeInTheDocument()
    window.history.pushState({}, '', '/')
  })
})
