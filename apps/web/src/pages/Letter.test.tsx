import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { analysis } from '../test/fixtures'
import { letterPath } from '../lib/letter'
import Letter from './Letter'

const json = (body: unknown, status = 200) =>
  Promise.resolve(new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } }))

const TEXT =
  'Olá. Sou cliente da Vivaz Telecom e contesto cobranças da fatura de outubro de 2026 (vencimento em 10/11/2026).\n\nItens contestados:\n- Clube Vivaz Música: R$ 12,90. Serviço extra que você pode não ter contratado. Base legal: CDC art. 39, III.\n\nTotal contestado: R$ 12,90.\n\nPeço o cancelamento dessas cobranças.\n\nObrigado.'

function setup(url = letterPath('a1', ['TEL-01:i3'], '2026-123'), fetchImpl?: (u: string, i?: RequestInit) => Promise<Response>) {
  const fetchMock = vi.fn(
    fetchImpl ??
      ((u: string) =>
        u.endsWith('/dispute')
          ? json({ text: TEXT, total_cents: 1290, steps: [], deadlines: [], disclaimer: 'x', call_script: [] })
          : json(analysis)),
  )
  vi.stubGlobal('fetch', fetchMock)
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={[url]}>
        <Routes>
          <Route path="r/:id/carta" element={<Letter />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  )
  return fetchMock
}

beforeEach(() => vi.unstubAllGlobals())
afterEach(() => {
  cleanup()
  vi.unstubAllGlobals()
})

describe('Letter: carta formal em PDF', () => {
  it('monta a carta com destinatário, assunto, itens, base legal e linhas em branco', async () => {
    const f = setup()
    const carta = await screen.findByRole('article', { name: 'Carta de contestação' })
    expect(carta).toHaveTextContent('À Vivaz Telecom')
    // Carta formal: "Prezados," no lugar do "Olá." do texto para o chat.
    expect(carta).toHaveTextContent('Prezados, Sou cliente da Vivaz Telecom')
    expect(carta).not.toHaveTextContent('Olá')
    expect(carta).toHaveTextContent('Assunto: Contestação de cobranças da fatura de')
    expect(carta).toHaveTextContent('Clube Vivaz Música: R$ 12,90')
    expect(carta).toHaveTextContent('Base legal: CDC art. 39, III')
    expect(carta).toHaveTextContent('Local e data:')
    expect(carta).toHaveTextContent('Assinatura:')
    expect(carta).toHaveTextContent('Protocolo nº: 2026-123')
    // Nenhum dado pessoal vem preenchido: nome e assinatura ficam só com a linha em branco.
    expect(carta).not.toHaveTextContent(/CPF/i)

    const post = f.mock.calls.find(([u]) => String(u).endsWith('/dispute'))!
    expect(JSON.parse(post[1]!.body as string)).toEqual({ finding_ids: ['TEL-01:i3'], protocol: '2026-123' })
  })

  it('o botão "Baixar carta (PDF)" chama window.print', async () => {
    const print = vi.spyOn(window, 'print').mockImplementation(() => {})
    setup()
    await userEvent.click(await screen.findByRole('button', { name: 'Baixar carta (PDF)' }))
    expect(print).toHaveBeenCalledTimes(1)
    expect(screen.getByText(/Salvar como PDF/)).toBeInTheDocument()
    print.mockRestore()
  })

  it('sem protocolo, deixa o campo em branco para preencher', async () => {
    const f = setup(letterPath('a1', ['TEL-01:i3']))
    const carta = await screen.findByRole('article', { name: 'Carta de contestação' })
    expect(carta).not.toHaveTextContent('(protocolo nº')
    const post = f.mock.calls.find(([u]) => String(u).endsWith('/dispute'))!
    expect(JSON.parse(post[1]!.body as string)).toEqual({ finding_ids: ['TEL-01:i3'] })
  })

  it('sem itens na URL, explica e não chama a API', () => {
    const f = setup('/r/a1/carta')
    expect(screen.getByRole('heading', { name: 'Nenhum problema escolhido para a carta' })).toBeInTheDocument()
    expect(f).not.toHaveBeenCalled()
  })

  it('mostra o erro da API sem montar a carta', async () => {
    setup(letterPath('a1', ['TEL-01:i3']), (u) =>
      u.endsWith('/dispute') ? json({ error: { code: 'invalid', message: 'Escolha ao menos um problema.' } }, 400) : json(analysis),
    )
    expect(await screen.findByRole('alert')).toHaveTextContent('Escolha ao menos um problema.')
    expect(screen.queryByRole('article')).not.toBeInTheDocument()
  })
})
