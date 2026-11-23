import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import Layout from '../components/Layout'
import type { WhatsappSimReply } from '../lib/api'
import WhatsAppSim from './WhatsAppSim'

const ID = '3f2b8a1e-5c4d-4e6f-8a9b-0c1d2e3f4a5b'

const json = (body: unknown, status = 200) =>
  Promise.resolve(new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } }))

const respostaExemplo: WhatsappSimReply = {
  mensagens: [
    {
      texto: `Vivaz Telecom · outubro de 2026\nTotal da fatura: R$ 122,70\n\n⚠️ Achei 2 pontos para conferir.\n\nVeja tudo, com a lei e a carta pronta:\nhttps://humanobill.vercel.app/r/${ID}`,
      resultado_id: ID,
    },
  ],
}

type Handler = (path: string, body: unknown) => Promise<Response>

function mount(handler: Handler) {
  const fetchMock = vi.fn((url: string, init?: RequestInit) => {
    const path = url.replace(/^https?:\/\/[^/]+/, '')
    return handler(path, init?.body ? JSON.parse(init.body as string) : undefined)
  })
  vi.stubGlobal('fetch', fetchMock)
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={['/simulador-whatsapp']}>
        <Routes>
          <Route element={<Layout />}>
            <Route path="simulador-whatsapp" element={<WhatsAppSim />} />
            <Route path="r/:id" element={<h1>Resultado {ID.slice(0, 4)}</h1>} />
          </Route>
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  )
  return fetchMock
}

/** Chamadas ao simulador (ignora o `/health` do warm-up do Layout). */
const simCalls = (m: ReturnType<typeof vi.fn>) =>
  m.mock.calls.filter(([u]) => String(u).includes('/whatsapp/simulador'))

beforeEach(() => vi.unstubAllGlobals())
afterEach(() => vi.unstubAllGlobals())

describe('#58: simulador do bot de WhatsApp', () => {
  it('explica que é simulação, sem marca, e oferece log, exemplos e campo de texto', () => {
    mount(() => json({}))
    expect(screen.getByRole('heading', { level: 1, name: 'Simulador de conversa' })).toBeInTheDocument()
    expect(screen.getByText(/Simulação: o bot real precisa de um número aprovado na Meta/)).toBeInTheDocument()
    const log = screen.getByRole('log', { name: 'Mensagens da conversa' })
    expect(log).toHaveAttribute('aria-live', 'polite')
    expect(within(log).getByText(/Nenhuma mensagem ainda/)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Conta de luz alta e leitura estimada' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Celular com serviços extras' })).toBeInTheDocument()
    expect(screen.getByLabelText('Escreva uma mensagem')).toBeInTheDocument()
    // Nada de arquivo: o simulador não gasta a IA.
    expect(document.querySelector('input[type="file"]')).toBeNull()
    // Sem marca do WhatsApp nem da Meta como identidade visual.
    expect(screen.queryByAltText(/whatsapp|meta/i)).toBeNull()
    expect(screen.queryByRole('img')).toBeNull()
  })

  it('botão de exemplo manda o id do exemplo e mostra a pergunta e a resposta no log', async () => {
    const user = userEvent.setup()
    const fetchMock = mount((path) => (path === '/whatsapp/simulador' ? json(respostaExemplo) : json({})))
    await user.click(screen.getByRole('button', { name: 'Celular com serviços extras' }))
    const log = screen.getByRole('log')
    expect(await within(log).findByText(/Vivaz Telecom · outubro de 2026/, undefined, { timeout: 3000 })).toBeInTheDocument()
    expect(within(log).getByText('Exemplo: Celular com serviços extras')).toBeInTheDocument()
    const [call] = simCalls(fetchMock)
    expect(call[1]).toMatchObject({ method: 'POST' })
    expect(JSON.parse((call[1] as RequestInit).body as string)).toEqual({ exemplo: 'telecom-1' })
    // Cada mensagem tem horário.
    expect(log.querySelectorAll('time').length).toBe(2)
  })

  it('o link do resultado abre /r/<id> dentro do app', async () => {
    const user = userEvent.setup()
    mount(() => json(respostaExemplo))
    await user.click(screen.getByRole('button', { name: 'Celular com serviços extras' }))
    const link = await screen.findByRole('link', { name: `https://humanobill.vercel.app/r/${ID}` }, { timeout: 3000 })
    expect(link).toHaveAttribute('href', `/r/${ID}`)
    await user.click(link)
    expect(await screen.findByRole('heading', { name: /Resultado 3f2b/ })).toBeInTheDocument()
  })

  it('texto digitado vai como texto, limpa o campo e mostra a resposta do bot', async () => {
    const user = userEvent.setup()
    const fetchMock = mount(() => json({ mensagens: [{ texto: 'Oi! Eu sou o HumanoBill.', resultado_id: null }] }))
    const campo = screen.getByLabelText('Escreva uma mensagem')
    const enviar = screen.getByRole('button', { name: 'Enviar' })
    expect(enviar).toBeDisabled()
    await user.type(campo, '  menu  ')
    await user.click(enviar)
    expect(campo).toHaveValue('')
    expect(await screen.findByText('Oi! Eu sou o HumanoBill.', undefined, { timeout: 3000 })).toBeInTheDocument()
    expect(screen.getByText('menu')).toBeInTheDocument()
    expect(JSON.parse((simCalls(fetchMock)[0][1] as RequestInit).body as string)).toEqual({ texto: 'menu' })
  })

  it('mostra "digitando…" e trava os botões enquanto espera a resposta', async () => {
    const user = userEvent.setup()
    let liberar: (r: Response) => void = () => {}
    mount(() => new Promise<Response>((r) => (liberar = r)))
    await user.click(screen.getByRole('button', { name: 'Ver o menu' }))
    expect(await screen.findByText('HumanoBill está digitando…')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Ver o menu' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Celular com serviços extras' })).toBeDisabled()
    liberar(new Response(JSON.stringify({ mensagens: [{ texto: 'Pronto.', resultado_id: null }] }), { status: 200 }))
    expect(await screen.findByText('Pronto.', undefined, { timeout: 3000 })).toBeInTheDocument()
    await waitFor(() => expect(screen.queryByText('HumanoBill está digitando…')).toBeNull())
    expect(screen.getByRole('button', { name: 'Ver o menu' })).toBeEnabled()
  })

  it('erro da API aparece como alerta amigável e a conversa continua possível', async () => {
    const user = userEvent.setup()
    mount(() =>
      json({ error: { code: 'rate_limited', message: 'Muitas tentativas seguidas, tente em alguns minutos' } }, 429),
    )
    await user.click(screen.getByRole('button', { name: 'Ver o menu' }))
    const alerta = await screen.findByRole('alert', undefined, { timeout: 3000 })
    expect(alerta).toHaveTextContent('Muitas tentativas seguidas, tente em alguns minutos')
    expect(screen.getByRole('button', { name: 'Ver o menu' })).toBeEnabled()
  })

  it('o rodapé tem um link discreto para o simulador', () => {
    mount(() => json({}))
    const rodape = screen.getByRole('navigation', { name: 'Rodapé' })
    expect(within(rodape).getByRole('link', { name: 'Simulador de conversa' })).toHaveAttribute('href', '/simulador-whatsapp')
  })
})
