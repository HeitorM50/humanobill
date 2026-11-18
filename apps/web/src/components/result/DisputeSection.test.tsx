import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { analysis } from '../../test/fixtures'
import { DisputeSection } from './DisputeSection'

const callScript = [
  { title: 'Apresente-se', say: 'Olá. Sou cliente da Vivaz Telecom e estou ligando para contestar cobranças.' },
  { title: 'Diga o mês da fatura', say: 'Quero falar da fatura de outubro de 2026, com vencimento em 10/11/2026.' },
  { title: 'Cite o item 1 de 1', say: 'Tem uma cobrança de "Clube Vivaz Música", no valor de R$ 12,90.' },
  { title: 'Peça o estorno', say: 'O total contestado é R$ 12,90. Peço o cancelamento dessas cobranças.' },
  { title: 'Peça o número do protocolo', say: 'Por favor, me informe o número do protocolo deste atendimento.' },
]

function disputeFor(protocol?: string) {
  return {
    text: protocol ? `Contesto o item. Este pedido complementa o atendimento de protocolo ${protocol}.` : 'Contesto o item. Peço o número de protocolo deste atendimento.',
    total_cents: 1290,
    steps: [{ title: 'SAC da operadora', description: 'Ligue e anote o protocolo.', url: null }],
    deadlines: ['A operadora tem 30 dias para responder'],
    disclaimer: 'Conteúdo informativo, não é orientação jurídica.',
    call_script: callScript,
  }
}

function setup() {
  const fetchMock = vi.fn((_url: string, init?: RequestInit) => {
    const body = JSON.parse(init!.body as string) as { protocol?: string }
    return Promise.resolve(new Response(JSON.stringify(disputeFor(body.protocol)), { status: 200, headers: { 'Content-Type': 'application/json' } }))
  })
  vi.stubGlobal('fetch', fetchMock)
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  render(
    <QueryClientProvider client={qc}>
      <DisputeSection analysis={analysis} selectedIds={['TEL-01:i3']} />
    </QueryClientProvider>,
  )
  return fetchMock
}

const postBodies = (m: ReturnType<typeof setup>) => m.mock.calls.map(([, i]) => JSON.parse(i!.body as string))

beforeEach(() => vi.unstubAllGlobals())
afterEach(() => vi.unstubAllGlobals())

describe('DisputeSection: Prefiro ligar (RF17)', () => {
  // Cenário "Roteiro com os achados"
  it('mostra o roteiro numerado, na ordem, e o campo para anotar o protocolo', async () => {
    setup()
    await userEvent.click(screen.getByRole('button', { name: 'Prefiro ligar' }))
    expect(screen.getByRole('button', { name: 'Prefiro ligar' })).toHaveAttribute('aria-pressed', 'true')
    await userEvent.click(screen.getByRole('button', { name: 'Gerar roteiro' }))

    const lista = await screen.findByRole('list', { name: 'Roteiro da ligação' })
    const passos = within(lista).getAllByRole('listitem')
    expect(passos).toHaveLength(callScript.length)
    callScript.forEach((p, i) => {
      expect(within(passos[i]).getByRole('heading', { level: 4, name: p.title })).toBeInTheDocument()
      expect(passos[i]).toHaveTextContent(p.say)
    })
    expect(screen.getByLabelText('Anote o protocolo aqui')).toBeInTheDocument()
  })

  it('copia o roteiro inteiro, passo a passo', async () => {
    const writeText = vi.fn(() => Promise.resolve())
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true })
    setup()
    await userEvent.click(screen.getByRole('button', { name: 'Prefiro ligar' }))
    await userEvent.click(screen.getByRole('button', { name: 'Gerar roteiro' }))
    await userEvent.click(await screen.findByRole('button', { name: 'Copiar roteiro' }))
    const copiado = (writeText.mock.calls[0] as unknown as [string])[0]
    expect(copiado).toContain('1. Apresente-se')
    expect(copiado).toContain('5. Peça o número do protocolo')
    expect(copiado).toContain('Tem uma cobrança de "Clube Vivaz Música"')
  })

  // Cenário "Protocolo anotado aparece na contestação"
  it('refaz a contestação com o protocolo anotado', async () => {
    const fetchMock = setup()
    await userEvent.click(screen.getByRole('button', { name: 'Prefiro ligar' }))
    await userEvent.click(screen.getByRole('button', { name: 'Gerar roteiro' }))
    await userEvent.type(await screen.findByLabelText('Anote o protocolo aqui'), '2026111012345')
    await userEvent.click(screen.getByRole('button', { name: 'Refazer contestação com o protocolo' }))

    expect(await screen.findByText(/protocolo 2026111012345\./)).toBeInTheDocument()
    expect(postBodies(fetchMock).at(-1)).toEqual({ finding_ids: ['TEL-01:i3'], protocol: '2026111012345' })
  })

  it('não deixa refazer com protocolo que tem letras e explica o motivo', async () => {
    setup()
    await userEvent.click(screen.getByRole('button', { name: 'Prefiro ligar' }))
    await userEvent.click(screen.getByRole('button', { name: 'Gerar roteiro' }))
    await userEvent.type(await screen.findByLabelText('Anote o protocolo aqui'), 'ABC123')
    expect(screen.getByText('Use só números e hífen (até 30).')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Refazer contestação com o protocolo' })).toBeDisabled()
  })

  it('o modo "Prefiro escrever" continua igual: sem roteiro, com o botão de sempre', async () => {
    setup()
    expect(screen.getByRole('button', { name: 'Prefiro escrever' })).toHaveAttribute('aria-pressed', 'true')
    await userEvent.click(screen.getByRole('button', { name: 'Gerar contestação' }))
    expect(await screen.findByText('Seu texto pronto')).toBeInTheDocument()
    expect(screen.queryByRole('list', { name: 'Roteiro da ligação' })).toBeNull()
  })
})
