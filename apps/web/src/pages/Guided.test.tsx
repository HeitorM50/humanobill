import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { cleanup, render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import Layout from '../components/Layout'
import { readHistory, recordAnalysis } from '../lib/history'
import { analysis as base, energyAnalysis } from '../test/fixtures'
import type { Analysis } from '../types/Analysis'
import Guided from './Guided'
import Landing from './Landing'

/*
 * RF22 — Guiar análise (modo passo a passo). Um teste por cenário Gherkin de docs/02-requisitos.md,
 * mais foco, "Passo X de Y", voltar sem perder respostas, etapa sem achados e extras pulados.
 */

const json = (body: unknown, status = 200) =>
  Promise.resolve(new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } }))

type Handler = (url: string, init?: RequestInit) => Promise<Response>
type Extra = (url: string, init?: RequestInit) => Promise<Response> | undefined

const callScript = [
  { title: 'Apresente-se', say: 'Olá. Sou cliente da Vivaz Telecom e estou ligando para contestar cobranças.' },
  { title: 'Peça o estorno', say: 'O total contestado é R$ 12,90. Peço o cancelamento dessas cobranças.' },
]
const dispute = {
  text: 'Contesto a cobrança do Clube Vivaz Música.',
  total_cents: 1290,
  steps: [{ title: 'SAC da operadora', description: 'Ligue e anote o protocolo.', url: null }],
  deadlines: ['A operadora tem 30 dias para responder'],
  disclaimer: 'Conteúdo informativo, não é orientação jurídica.',
  call_script: callScript,
}

// Três perguntas de tipos diferentes e dois achados (um indevido, um suspeito).
const three: Analysis = {
  ...base,
  id: 'a1',
  findings: [
    { ...base.findings[0], severity: 'indevido', title: 'Serviço extra que você pode não ter contratado' },
    {
      ...base.findings[0],
      id: 'TEL-03:i1',
      rule_id: 'TEL-03',
      severity: 'suspeito',
      title: 'Juros acima do permitido',
      explanation: 'Os juros parecem maiores que o limite da lei.',
      item_refs: ['i1'],
      amount_cents: 500,
    },
  ],
  questions: [
    { id: 'TEL-01:i3', rule_id: 'TEL-01', item_id: 'i3', text: 'Você contratou Clube Vivaz Música?', kind: 'yes_no' },
    { id: 'TEL-03:i1', rule_id: 'TEL-03', item_id: 'i1', text: 'Quanto você pagou de juros?', kind: 'amount' },
    { id: 'TEL-05:d', rule_id: 'TEL-05', item_id: null, text: 'Quando a fatura chegou na sua casa?', kind: 'date' },
  ],
}

const clean: Analysis = { ...base, id: 'ok1', findings: [], questions: [], answers: {}, paid: null }

/** Servidor falso: guarda o estado da análise e imita GET, PATCH, contestação e lembrete. */
function server(initial: Analysis, extra?: Extra) {
  let current = initial
  const handler: Handler = (url, init) => {
    if (extra) {
      const r = extra(url, init)
      if (r) return r
    }
    const method = init?.method ?? 'GET'
    if (url.includes('/demo/')) return json(current)
    if (url.endsWith('/dispute')) return json(dispute)
    if (url.endsWith('/reminders')) return json({ due_at: '2026-12-10T12:00:00Z' })
    if (method === 'PATCH') {
      const body = JSON.parse(init!.body as string) as { answers?: Analysis['answers']; paid?: boolean | null }
      current = { ...current, ...(body.answers ? { answers: body.answers } : {}), ...('paid' in body ? { paid: body.paid ?? null } : {}) }
      return json(current)
    }
    if (method === 'POST' && url.endsWith('/analyses')) return json(current)
    return json(current)
  }
  return handler
}

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
            <Route path="guiado" element={<Guided />} />
            <Route path="guiado/:id" element={<Guided />} />
            <Route path="r/:id" element={<h1>Versão completa</h1>} />
            <Route path="analisar" element={<h1>Enviar minha fatura</h1>} />
            <Route path="comparar" element={<h1>Comparar duas faturas</h1>} />
          </Route>
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  )
  return fetchMock
}

const calls = (m: ReturnType<typeof mount>, method: string, suffix = '') =>
  m.mock.calls.filter(([u, i]) => (i?.method ?? 'GET') === method && String(u).endsWith(suffix))
const bodyOf = (call: unknown[]) => JSON.parse((call[1] as RequestInit).body as string)

const title = () => screen.getByRole('heading', { level: 1 })
const next = () => userEvent.click(screen.getByRole('button', { name: 'Próximo' }))
const back = () => userEvent.click(screen.getByRole('button', { name: 'Voltar' }))
const stepText = () => screen.getByTestId('passo').textContent

/** Espera o título da tela aparecer e devolve o título. */
const onScreen = async (name: string | RegExp) => screen.findByRole('heading', { level: 1, name })

beforeEach(() => vi.unstubAllGlobals())
afterEach(() => {
  cleanup() // desmonta antes de tirar a voz falsa: o "Ouvir" cancela a fala ao sair da tela
  vi.unstubAllGlobals()
})

describe('RF22: entrada pela página inicial', () => {
  it('a landing tem a opção bem visível "Prefiro passo a passo" para /guiado', () => {
    mount(server(three), '/')
    expect(screen.getByRole('link', { name: 'Prefiro passo a passo' })).toHaveAttribute('href', '/guiado')
  })

  it('há um jeito de sair para a versão completa, que continua em /r/:id', async () => {
    mount(server(three), '/guiado/a1')
    await onScreen(/Pergunta 1 de 3/)
    expect(screen.getByRole('link', { name: /versão completa/i })).toHaveAttribute('href', '/r/a1')
  })

  it('sem análise, a saída leva à versão completa de envio', () => {
    mount(server(three), '/guiado')
    expect(screen.getByRole('link', { name: /versão completa/i })).toHaveAttribute('href', '/analisar')
  })
})

describe('RF22: Cenário "Fluxo guiado completo"', () => {
  it('avança com Próximo: enviar, confirmar itens, ver resultado, conhecer e contestar; e volta em qualquer etapa', async () => {
    const m = mount(server(three), '/guiado')

    // 1. Enviar (aqui, com um exemplo)
    expect(title()).toHaveTextContent('Vamos conferir sua fatura')
    expect(stepText()).toBe('Passo 1 de 5')
    await userEvent.click(screen.getByRole('button', { name: 'Usar uma fatura de exemplo' }))
    await userEvent.click(screen.getByRole('button', { name: /Celular com serviços extras/ }))
    expect(calls(m, 'POST', '/analyses/demo/telecom-1')).toHaveLength(1)

    // 2. Confirmar itens (uma pergunta por tela)
    await onScreen('Pergunta 1 de 3')
    expect(stepText()).toBe('Passo 2 de 5')
    await next()
    expect(title()).toHaveTextContent('Pergunta 2 de 3')
    await next()
    expect(title()).toHaveTextContent('Pergunta 3 de 3')
    await next()
    expect(title()).toHaveTextContent('Você já pagou esta fatura?')
    await next()

    // 3. Ver o resultado
    expect(title()).toHaveTextContent('Veja o resultado')
    expect(stepText()).toBe('Passo 3 de 5')
    await next()
    expect(title()).toHaveTextContent('Problema 1 de 2')
    await next()
    expect(title()).toHaveTextContent('Problema 2 de 2')
    await next()

    // 4. Conhecer a conta (extras)
    expect(stepText()).toBe('Passo 4 de 5')
    expect(title()).toHaveTextContent('Conheça sua conta')
    await next()
    expect(title()).toHaveTextContent('Quer um lembrete por e-mail?')
    await next()
    expect(title()).toHaveTextContent('Quer comparar com outra fatura?')
    await next()

    // 5. Contestar
    expect(stepText()).toBe('Passo 5 de 5')
    expect(title()).toHaveTextContent('O que você quer contestar?')
    await next()
    expect(await onScreen('Seu texto está pronto')).toBeInTheDocument()
    expect(screen.getByText(dispute.text)).toBeInTheDocument()
    expect(screen.getByRole('link', { name: /Carta formal \(PDF\)/ })).toHaveAttribute('href', expect.stringMatching(/^\/r\/a1\/carta\?item=/))
    await next()
    expect(title()).toHaveTextContent('Se a empresa não resolver')
    await next()
    expect(title()).toHaveTextContent('Prefere resolver por telefone?')
    expect(screen.getByRole('list', { name: 'Roteiro da ligação' })).toBeInTheDocument()

    // Voltar em qualquer etapa, até o começo da análise.
    await back()
    expect(title()).toHaveTextContent('Se a empresa não resolver')
    await back()
    await back()
    expect(title()).toHaveTextContent('O que você quer contestar?')
    await back()
    expect(title()).toHaveTextContent('Quer comparar com outra fatura?')
    await back()
    await back()
    expect(title()).toHaveTextContent('Conheça sua conta')
    await back()
    expect(title()).toHaveTextContent('Problema 2 de 2')
    await back()
    await back()
    expect(title()).toHaveTextContent('Veja o resultado')
    await back()
    expect(title()).toHaveTextContent('Você já pagou esta fatura?')
    await back()
    await back()
    await back()
    expect(title()).toHaveTextContent('Pergunta 1 de 3')
    await back()
    expect(title()).toHaveTextContent('Sua fatura foi enviada')
    expect(stepText()).toBe('Passo 1 de 5')
  })

  it('o primeiro passo também aceita uma fatura enviada, com consentimento', async () => {
    const m = mount(server(three), '/guiado')
    await userEvent.click(screen.getByRole('button', { name: 'Enviar a minha fatura' }))
    const analisar = screen.getByRole('button', { name: 'Analisar' })
    expect(analisar).toBeDisabled()

    const file = new File(['%PDF'], 'fatura.pdf', { type: 'application/pdf' })
    await userEvent.upload(screen.getByLabelText('Arquivo da fatura'), file)
    expect(screen.getByText(/fatura\.pdf/)).toBeInTheDocument()
    expect(analisar).toBeDisabled() // falta o consentimento
    await userEvent.click(screen.getByRole('checkbox', { name: 'Entendi e concordo' }))
    expect(analisar).toBeEnabled()
    await userEvent.click(analisar)

    await onScreen('Pergunta 1 de 3')
    const post = calls(m, 'POST', '/analyses')[0]
    const form = (post[1] as RequestInit).body as FormData
    expect(form.get('consent')).toBe('true')
    expect((form.get('file') as File).name).toBe('fatura.pdf')
  })

  it('recusa arquivo grande demais, com a mesma frase da versão completa', async () => {
    mount(server(three), '/guiado')
    await userEvent.click(screen.getByRole('button', { name: 'Enviar a minha fatura' }))
    const big = new File(['x'], 'grande.pdf', { type: 'application/pdf' })
    Object.defineProperty(big, 'size', { value: 11 * 1024 * 1024 })
    await userEvent.upload(screen.getByLabelText('Arquivo da fatura'), big)
    expect(await screen.findByRole('alert')).toHaveTextContent('O arquivo deve ter até 10 MB')
    expect(screen.getByRole('button', { name: 'Analisar' })).toBeDisabled()
  })
})

describe('RF22: Cenário "Uma pergunta por vez"', () => {
  it('com 3 perguntas, cada uma aparece sozinha na tela', async () => {
    mount(server(three), '/guiado/a1')
    await onScreen('Pergunta 1 de 3')
    expect(screen.getByText('Você contratou Clube Vivaz Música?')).toBeInTheDocument()
    expect(screen.queryByText('Quanto você pagou de juros?')).toBeNull()
    expect(screen.queryByText('Quando a fatura chegou na sua casa?')).toBeNull()

    await next()
    expect(screen.getByLabelText('Quanto você pagou de juros?')).toBeInTheDocument()
    expect(screen.queryByText('Você contratou Clube Vivaz Música?')).toBeNull()
    expect(screen.queryByLabelText('Quando a fatura chegou na sua casa?')).toBeNull()

    await next()
    expect(screen.getByLabelText('Quando a fatura chegou na sua casa?')).toBeInTheDocument()
    expect(screen.queryByLabelText('Quanto você pagou de juros?')).toBeNull()
  })

  it('responder usa o mesmo PATCH da versão completa: o mapa inteiro de respostas', async () => {
    const m = mount(server(three), '/guiado/a1')
    await onScreen('Pergunta 1 de 3')
    await userEvent.click(screen.getByRole('button', { name: 'Contratei' }))
    await waitFor(() => expect(calls(m, 'PATCH')).toHaveLength(1))
    expect(bodyOf(calls(m, 'PATCH')[0])).toEqual({ answers: { 'TEL-01:i3': { type: 'yes' } } })

    await next()
    await userEvent.type(screen.getByLabelText('Quanto você pagou de juros?'), '500')
    await userEvent.click(screen.getByRole('button', { name: 'Confirmar' }))
    await waitFor(() => expect(calls(m, 'PATCH')).toHaveLength(2))
    expect(bodyOf(calls(m, 'PATCH')[1])).toEqual({
      answers: { 'TEL-01:i3': { type: 'yes' }, 'TEL-03:i1': { type: 'amount', value: 500 } },
    })
  })

  it('a pergunta "paguei / não paguei" também fica sozinha e manda { paid }', async () => {
    const m = mount(server(three), '/guiado/a1')
    await onScreen('Pergunta 1 de 3')
    await next()
    await next()
    await next()
    expect(await onScreen('Você já pagou esta fatura?')).toBeInTheDocument()
    expect(screen.queryByText('Quando a fatura chegou na sua casa?')).toBeNull()
    await userEvent.click(screen.getByRole('button', { name: 'Já paguei' }))
    await waitFor(() => expect(calls(m, 'PATCH')).toHaveLength(1))
    expect(bodyOf(calls(m, 'PATCH')[0])).toEqual({ paid: true })
    expect(await screen.findByRole('button', { name: 'Já paguei' })).toHaveAttribute('aria-pressed', 'true')
    expect(screen.getByRole('button', { name: 'Ainda não paguei' })).toHaveAttribute('aria-pressed', 'false')
  })
})

describe('RF22: voltar sem perder o que foi respondido', () => {
  it('as respostas continuam marcadas ao voltar', async () => {
    mount(server(three), '/guiado/a1')
    await onScreen('Pergunta 1 de 3')
    await userEvent.click(screen.getByRole('button', { name: 'Não contratei' }))
    await waitFor(() => expect(screen.getByRole('button', { name: 'Não contratei' })).toHaveAttribute('aria-pressed', 'true'))
    await next()
    await back()
    expect(screen.getByRole('button', { name: 'Não contratei' })).toHaveAttribute('aria-pressed', 'true')
    expect(screen.getByRole('button', { name: 'Contratei' })).toHaveAttribute('aria-pressed', 'false')
  })

  it('a escolha do que contestar continua ao voltar', async () => {
    mount(server(three), '/guiado/a1')
    await onScreen('Pergunta 1 de 3')
    for (let i = 0; i < 4; i++) await next() // 3 perguntas + "pagou"
    await next() // resultado -> problema 1
    await next() // problema 2
    await next() // conhecer
    await next() // lembrete
    await next() // comparar
    await next() // contestar
    await onScreen('O que você quer contestar?')
    const suspeito = screen.getByRole('checkbox', { name: /Juros acima do permitido/ })
    expect(suspeito).not.toBeChecked()
    await userEvent.click(suspeito)
    await next()
    await onScreen('Seu texto está pronto')
    await back()
    expect(screen.getByRole('checkbox', { name: /Juros acima do permitido/ })).toBeChecked()
    expect(screen.getByRole('checkbox', { name: /Serviço extra/ })).toBeChecked()
  })
})

describe('RF22: acessibilidade', () => {
  it('"Passo X de Y" está em texto e o foco vai para o título a cada mudança', async () => {
    mount(server(three), '/guiado/a1')
    await onScreen('Pergunta 1 de 3')
    expect(stepText()).toBe('Passo 2 de 5')

    await next()
    expect(stepText()).toBe('Passo 2 de 5')
    expect(title()).toHaveTextContent('Pergunta 2 de 3')
    expect(title()).toHaveFocus()

    await back()
    expect(title()).toHaveTextContent('Pergunta 1 de 3')
    expect(title()).toHaveFocus()
  })

  it('anuncia a tela nova em uma região aria-live educada', async () => {
    mount(server(three), '/guiado/a1')
    await onScreen('Pergunta 1 de 3')
    const live = screen.getByTestId('anuncio')
    expect(live).toHaveAttribute('aria-live', 'polite')
    await next()
    await waitFor(() => expect(live).toHaveTextContent('Passo 2 de 5: Pergunta 2 de 3'))
  })

  it('tem os botões Próximo e Voltar grandes (56 px ou mais)', async () => {
    mount(server(three), '/guiado/a1')
    await onScreen('Pergunta 1 de 3')
    for (const name of ['Próximo', 'Voltar']) {
      expect(screen.getByRole('button', { name }).className).toContain('min-h-14')
    }
    expect(screen.getByRole('button', { name: 'Contratei' }).className).toContain('min-h-14')
  })

  it('funciona só com o teclado: Tab chega em Próximo e Enter avança', async () => {
    mount(server(three), '/guiado/a1')
    await onScreen('Pergunta 1 de 3')
    const proximo = screen.getByRole('button', { name: 'Próximo' })
    for (let i = 0; i < 12 && document.activeElement !== proximo; i++) await userEvent.tab()
    expect(proximo).toHaveFocus()
    await userEvent.keyboard('{Enter}')
    expect(title()).toHaveTextContent('Pergunta 2 de 3')
  })
})

describe('RF22: etapa sem achados', () => {
  it('mostra "tudo certo", pula a etapa de contestar e conta um passo a menos', async () => {
    mount(server(clean), '/guiado/ok1')
    // Sem perguntas e sem problemas, a etapa 2 só avisa e deixa seguir.
    expect(await onScreen('Nenhuma pergunta por enquanto')).toBeInTheDocument()
    expect(stepText()).toBe('Passo 2 de 4')
    await next()

    expect(title()).toHaveTextContent('Veja o resultado')
    expect(screen.getByText('Tudo certo com esta conta')).toBeInTheDocument()
    expect(stepText()).toBe('Passo 3 de 4')
    await next()

    expect(stepText()).toBe('Passo 4 de 4')
    // Sem problemas não há lembrete nem contestação.
    expect(screen.queryByRole('heading', { name: /lembrete/i })).toBeNull()
    expect(title()).toHaveTextContent('Conheça sua conta')
    await next()
    expect(title()).toHaveTextContent('Quer comparar com outra fatura?')
    expect(screen.queryByRole('heading', { name: /contestar/i })).toBeNull()
    expect(screen.queryByRole('button', { name: 'Próximo' })).toBeNull()
    expect(screen.getByRole('link', { name: 'Terminar' })).toHaveAttribute('href', '/')
  })

  it('conta de luz mostra o consumo na etapa de conhecer a conta', async () => {
    mount(server({ ...energyAnalysis, answers: {}, paid: null }), '/guiado/e1')
    await onScreen('Nenhuma pergunta por enquanto')
    await next() // resultado
    await next() // conheça
    await next() // consumo
    expect(screen.getByRole('heading', { name: 'Seu consumo de luz' })).toBeInTheDocument()
  })
})

describe('RF22: extras opcionais e puláveis', () => {
  it('passar direto pelos extras não agenda lembrete nem abre a comparação', async () => {
    const m = mount(server(three), '/guiado/a1')
    await onScreen('Pergunta 1 de 3')
    for (let i = 0; i < 7; i++) await next() // 3 perguntas + pagou + resultado + 2 problemas
    expect(title()).toHaveTextContent('Conheça sua conta')
    expect(screen.getByRole('heading', { name: 'Para onde vai seu dinheiro' })).toBeInTheDocument()
    await next()
    expect(title()).toHaveTextContent('Quer um lembrete por e-mail?')
    expect(screen.getByText(/Opcional/)).toBeInTheDocument()
    await next()
    expect(calls(m, 'POST', '/reminders')).toHaveLength(0)
    expect(screen.getByRole('link', { name: 'Comparar duas faturas' })).toHaveAttribute('href', '/comparar?a=a1')
    await next()
    expect(title()).toHaveTextContent('O que você quer contestar?')
  })

  it('quem quiser agenda o lembrete na própria tela', async () => {
    const m = mount(server(three), '/guiado/a1')
    await onScreen('Pergunta 1 de 3')
    for (let i = 0; i < 7; i++) await next()
    await onScreen('Conheça sua conta')
    await next()
    await userEvent.type(screen.getByLabelText('Seu e-mail'), 'ana@exemplo.com.br')
    await userEvent.click(screen.getByRole('checkbox'))
    await userEvent.click(screen.getByRole('button', { name: 'Me lembrar' }))
    expect(await screen.findByText(/Vamos te avisar em/)).toBeInTheDocument()
    expect(calls(m, 'POST', '/reminders')).toHaveLength(1)
    expect(bodyOf(calls(m, 'POST', '/reminders')[0])).toMatchObject({ analysis_id: 'a1', email: 'ana@exemplo.com.br', consent: true })
  })
})

describe('RF22: resultado e contestação', () => {
  it('mostra o total e o valor que parece indevido, vindos da API', async () => {
    mount(server(three), '/guiado/a1')
    await onScreen('Pergunta 1 de 3')
    for (let i = 0; i < 4; i++) await next()
    await onScreen('Veja o resultado')
    expect(screen.getByText('R$ 129,80')).toBeInTheDocument() // total da fatura
    expect(screen.getByText('R$ 12,90')).toBeInTheDocument() // só o achado "indevido"
    expect(screen.getByText(/parecem indevidos/)).toBeInTheDocument()
    await next()
    expect(screen.getByRole('heading', { level: 2, name: 'Serviço extra que você pode não ter contratado' })).toBeInTheDocument()
    expect(screen.getByText('Indevido')).toBeInTheDocument()
  })

  it('o botão Ouvir lê o resultado, quando o navegador tem voz', async () => {
    class Utterance {
      lang = ''
      text: string
      constructor(text: string) {
        this.text = text
      }
    }
    const speak = vi.fn()
    vi.stubGlobal('speechSynthesis', { speak, cancel: vi.fn() })
    vi.stubGlobal('SpeechSynthesisUtterance', Utterance)
    mount(server(three), '/guiado/a1')
    await onScreen('Pergunta 1 de 3')
    for (let i = 0; i < 4; i++) await next()
    await onScreen('Veja o resultado')
    await userEvent.click(screen.getByRole('button', { name: 'Ouvir' }))
    expect(speak).toHaveBeenCalled()
  })

  it('contesta só o que foi marcado, gera o texto e deixa copiar', async () => {
    const writeText = vi.fn(() => Promise.resolve())
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true })
    const m = mount(server(three), '/guiado/a1')
    await onScreen('Pergunta 1 de 3')
    for (let i = 0; i < 10; i++) await next()
    await onScreen('O que você quer contestar?')
    await next()
    await onScreen('Seu texto está pronto')

    const post = calls(m, 'POST', '/dispute')
    expect(post).toHaveLength(1)
    expect(bodyOf(post[0])).toEqual({ finding_ids: ['TEL-01:i3'] })

    await userEvent.click(screen.getByRole('button', { name: 'Copiar' }))
    expect(writeText).toHaveBeenCalledWith(dispute.text)
  })

  it('sem nada marcado, não deixa gerar o texto', async () => {
    mount(server(three), '/guiado/a1')
    await onScreen('Pergunta 1 de 3')
    for (let i = 0; i < 10; i++) await next()
    await onScreen('O que você quer contestar?')
    await userEvent.click(screen.getByRole('checkbox', { name: /Serviço extra/ }))
    expect(screen.getByText('Marque pelo menos um problema para continuar.')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Próximo' })).toBeDisabled()
  })

  it('mostra o erro da API ao gerar o texto e deixa tentar de novo', async () => {
    let fail = true
    const m = mount(
      server(three, (url) => (url.endsWith('/dispute') && fail ? json({ error: { code: 'x', message: 'Não deu para gerar agora.' } }, 500) : undefined)),
      '/guiado/a1',
    )
    await onScreen('Pergunta 1 de 3')
    for (let i = 0; i < 10; i++) await next()
    await onScreen('O que você quer contestar?')
    await next()
    expect(await screen.findByRole('alert')).toHaveTextContent('Não deu para gerar agora.')
    fail = false
    await userEvent.click(screen.getByRole('button', { name: 'Tentar de novo' }))
    expect(await screen.findByText(dispute.text)).toBeInTheDocument()
    expect(calls(m, 'POST', '/dispute')).toHaveLength(2)
  })

  it('o roteiro de ligação aparece na última tela, com o roteiro numerado', async () => {
    mount(server(three), '/guiado/a1')
    await onScreen('Pergunta 1 de 3')
    for (let i = 0; i < 10; i++) await next()
    await next() // gera
    await onScreen('Seu texto está pronto')
    await next()
    await onScreen('Se a empresa não resolver')
    expect(screen.getByText('Fique de olho nos prazos')).toBeInTheDocument()
    await next()
    await onScreen('Prefere resolver por telefone?')
    const lista = screen.getByRole('list', { name: 'Roteiro da ligação' })
    expect(within(lista).getAllByRole('listitem')).toHaveLength(callScript.length)
    expect(screen.getByRole('link', { name: 'Terminar' })).toHaveAttribute('href', '/')
  })
})

describe('RF22: erros ao abrir a análise', () => {
  it('404 avisa que não achou e oferece recomeçar o passo a passo', async () => {
    mount(() => json({ error: { code: 'not_found', message: 'Resultado não encontrado' } }, 404), '/guiado/x')
    expect(await screen.findByRole('heading', { level: 1, name: 'Resultado não encontrado' })).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'Começar de novo' })).toHaveAttribute('href', '/guiado')
  })

  it('410 avisa que expirou', async () => {
    mount(() => json({ error: { code: 'expired', message: 'Expirou' } }, 410), '/guiado/x')
    expect(await screen.findByRole('heading', { level: 1, name: 'Este resultado expirou. Analise de novo.' })).toBeInTheDocument()
  })
})

describe('RF22: o que foi digitado não se perde ao voltar', () => {
  /** Abre a tela do lembrete (depois de 7 telas de perguntas e resultado, e de "Conheça sua conta"). */
  async function openReminder(m: ReturnType<typeof mount>) {
    await onScreen('Pergunta 1 de 3')
    for (let i = 0; i < 7; i++) await next()
    await onScreen('Conheça sua conta')
    await next()
    await onScreen('Quer um lembrete por e-mail?')
    return m
  }

  it('o e-mail e o consentimento do lembrete continuam ao voltar e avançar', async () => {
    const m = mount(server(three), '/guiado/a1')
    await openReminder(m)
    await userEvent.type(screen.getByLabelText('Seu e-mail'), 'ana@exemplo.com.br')
    await userEvent.click(screen.getByRole('checkbox'))

    await back()
    await onScreen('Conheça sua conta')
    await next()
    await onScreen('Quer um lembrete por e-mail?')
    expect(screen.getByLabelText('Seu e-mail')).toHaveValue('ana@exemplo.com.br')
    expect(screen.getByRole('checkbox')).toBeChecked()
    expect(calls(m, 'POST', '/reminders')).toHaveLength(0)
  })

  it('depois de agendar, voltar e avançar mostra o aviso em vez de um formulário novo', async () => {
    const m = mount(server(three), '/guiado/a1')
    await openReminder(m)
    await userEvent.type(screen.getByLabelText('Seu e-mail'), 'ana@exemplo.com.br')
    await userEvent.click(screen.getByRole('checkbox'))
    await userEvent.click(screen.getByRole('button', { name: 'Me lembrar' }))
    expect(await screen.findByText(/Vamos te avisar em/)).toBeInTheDocument()

    await back()
    await onScreen('Conheça sua conta')
    await next()
    await onScreen('Quer um lembrete por e-mail?')
    expect(screen.getByText(/Vamos te avisar em/)).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Me lembrar' })).not.toBeInTheDocument()
    expect(calls(m, 'POST', '/reminders')).toHaveLength(1)
  })
})

describe('RF22: protocolo opcional na contestação', () => {
  async function openChoose(m: ReturnType<typeof mount>) {
    await onScreen('Pergunta 1 de 3')
    for (let i = 0; i < 10; i++) await next()
    await onScreen('O que você quer contestar?')
    return m
  }

  it('o protocolo digitado vai na contestação e no roteiro', async () => {
    const m = mount(server(three), '/guiado/a1')
    await openChoose(m)
    await userEvent.type(screen.getByLabelText('Já tem um protocolo? (opcional)'), '2026111012345')
    await next()
    await onScreen('Seu texto está pronto')
    expect(bodyOf(calls(m, 'POST', '/dispute')[0])).toEqual({ finding_ids: ['TEL-01:i3'], protocol: '2026111012345' })
  })

  it('sem protocolo, o pedido não leva o campo', async () => {
    const m = mount(server(three), '/guiado/a1')
    await openChoose(m)
    await next()
    await onScreen('Seu texto está pronto')
    expect(bodyOf(calls(m, 'POST', '/dispute')[0])).toEqual({ finding_ids: ['TEL-01:i3'] })
  })

  it('protocolo com letras mostra o erro e não deixa avançar', async () => {
    const m = mount(server(three), '/guiado/a1')
    await openChoose(m)
    await userEvent.type(screen.getByLabelText('Já tem um protocolo? (opcional)'), 'ABC123')
    expect(screen.getByText('Use só números e hífen (até 30).')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Próximo' })).toBeDisabled()
    expect(calls(m, 'POST', '/dispute')).toHaveLength(0)
  })

  it('o protocolo digitado continua ao voltar', async () => {
    const m = mount(server(three), '/guiado/a1')
    await openChoose(m)
    await userEvent.type(screen.getByLabelText('Já tem um protocolo? (opcional)'), '123-45')
    await next()
    await onScreen('Seu texto está pronto')
    await back()
    await onScreen('O que você quer contestar?')
    expect(screen.getByLabelText('Já tem um protocolo? (opcional)')).toHaveValue('123-45')
  })
})

describe('RF22: leitura ao vivo com IA no primeiro passo', () => {
  const pdfs = (url: string) => (url.includes('/exemplos/') ? Promise.resolve(new Response('%PDF', { status: 200 })) : undefined)

  it('com "Quero ver a IA lendo o PDF" marcado, o exemplo passa pelo envio real', async () => {
    const m = mount(server(three, pdfs), '/guiado')
    await userEvent.click(screen.getByRole('button', { name: 'Usar uma fatura de exemplo' }))
    await userEvent.click(screen.getByRole('checkbox', { name: /Quero ver a IA lendo o PDF/ }))
    await userEvent.click(screen.getByRole('button', { name: /Celular com serviços extras/ }))

    await onScreen('Pergunta 1 de 3')
    expect(calls(m, 'POST', '/analyses')).toHaveLength(1)
    expect(calls(m, 'POST').filter(([u]) => String(u).includes('/demo/'))).toHaveLength(0)
    const form = (calls(m, 'POST', '/analyses')[0][1] as RequestInit).body as FormData
    expect(form.get('consent')).toBe('true')
    expect((form.get('file') as File).name).toBe('telecom-1.pdf')
  })

  it('sem marcar, continua usando o exemplo pronto (sem IA)', async () => {
    const m = mount(server(three, pdfs), '/guiado')
    await userEvent.click(screen.getByRole('button', { name: 'Usar uma fatura de exemplo' }))
    await userEvent.click(screen.getByRole('button', { name: /Celular com serviços extras/ }))
    await onScreen('Pergunta 1 de 3')
    expect(calls(m, 'POST', '/analyses')).toHaveLength(0)
    expect(calls(m, 'POST').filter(([u]) => String(u).includes('/demo/'))).toHaveLength(1)
  })

  it('se a IA falhar, avisa e deixa usar o exemplo pronto', async () => {
    mount(
      server(three, (url, init) =>
        url.includes('/exemplos/')
          ? Promise.resolve(new Response('%PDF', { status: 200 }))
          : (init?.method ?? 'GET') === 'POST' && url.endsWith('/analyses')
            ? json({ error: { code: 'extraction_unavailable', message: 'A leitura por IA está indisponível agora.' } }, 503)
            : undefined,
      ),
      '/guiado',
    )
    await userEvent.click(screen.getByRole('button', { name: 'Usar uma fatura de exemplo' }))
    await userEvent.click(screen.getByRole('checkbox', { name: /Quero ver a IA lendo o PDF/ }))
    await userEvent.click(screen.getByRole('button', { name: /Celular com serviços extras/ }))
    expect(await screen.findByRole('alert')).toHaveTextContent(/indisponível agora.*Desmarque a leitura com IA/)
    expect(screen.getByRole('button', { name: /Celular com serviços extras/ })).toBeEnabled()
  })
})

describe('Minhas contas neste aparelho (modo guiado)', () => {
  beforeEach(() => window.localStorage.clear())

  it('guarda a conta ao abrir o passo a passo', async () => {
    mount(server(three), '/guiado/a1')
    await onScreen('Pergunta 1 de 3')
    expect(readHistory()).toHaveLength(1)
    expect(readHistory()[0]).toMatchObject({ id: 'a1', provider: 'Vivaz Telecom', month: '2026-10', dispute: null })
  })

  it('guarda o protocolo quando a contestação sai com protocolo', async () => {
    const m = mount(server(three), '/guiado/a1')
    await onScreen('Pergunta 1 de 3')
    for (let i = 0; i < 10; i++) await next()
    await onScreen('O que você quer contestar?')
    await userEvent.type(screen.getByLabelText('Já tem um protocolo? (opcional)'), '2026111012345')
    await next()
    await onScreen('Seu texto está pronto')
    expect(calls(m, 'POST', '/dispute')).toHaveLength(1)
    expect(readHistory()[0].dispute?.protocol).toBe('2026111012345')
  })

  it('sem protocolo, só a conta fica guardada', async () => {
    mount(server(three), '/guiado/a1')
    await onScreen('Pergunta 1 de 3')
    for (let i = 0; i < 10; i++) await next()
    await onScreen('O que você quer contestar?')
    await next()
    await onScreen('Seu texto está pronto')
    expect(readHistory()[0].dispute).toBeNull()
  })

  it.each([404, 410])('resultado que sumiu (%i) marca a conta como expirada', async (status) => {
    recordAnalysis(three)
    mount(() => json({ error: { code: 'gone', message: 'Sumiu.' } }, status), '/guiado/a1')
    await screen.findByRole('heading', { level: 1 })
    await waitFor(() => expect(readHistory()[0].expired).toBe(true))
  })
})
