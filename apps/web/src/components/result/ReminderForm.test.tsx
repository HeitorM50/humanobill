import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { ReminderForm } from './ReminderForm'

function json(body: unknown, status = 200) {
  return Promise.resolve(new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } }))
}

function setup(reply: () => Promise<Response> = () => json({ due_at: '2026-12-10T15:00:00+00:00' }, 201)) {
  const fetchMock = vi.fn((_url: string, _init?: RequestInit) => reply())
  vi.stubGlobal('fetch', fetchMock)
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  render(
    <QueryClientProvider client={qc}>
      <ReminderForm analysisId="a1" />
    </QueryClientProvider>,
  )
  return fetchMock
}

const botao = () => screen.getByRole('button', { name: 'Me lembrar' })
const campo = () => screen.getByLabelText('Seu e-mail')
const consentimento = () => screen.getByRole('checkbox', { name: /Aceito receber/ })

beforeEach(() => vi.unstubAllGlobals())
afterEach(() => vi.unstubAllGlobals())

describe('ReminderForm: Agendar lembrete (RF18)', () => {
  // Cenário "Lembrete agendado"
  it('com consentimento e e-mail válido, agenda e mostra a data em dd/mm/aaaa', async () => {
    const f = setup()
    await userEvent.click(consentimento())
    await userEvent.type(campo(), 'maria@exemplo.com')
    await userEvent.click(botao())

    expect(await screen.findByText('Vamos te avisar em 10/12/2026')).toBeInTheDocument()
    expect(f).toHaveBeenCalledTimes(1)
    const [url, init] = f.mock.calls[0]
    expect(url).toMatch(/\/reminders$/)
    expect(init?.method).toBe('POST')
    expect(JSON.parse(init!.body as string)).toEqual({ analysis_id: 'a1', email: 'maria@exemplo.com', consent: true })
  })

  // Cenário "E-mail inválido"
  it('e-mail inválido mostra "Informe um e-mail válido" e não chama a API', async () => {
    const f = setup()
    await userEvent.click(consentimento())
    await userEvent.type(campo(), 'maria@')
    await userEvent.click(botao())

    expect(screen.getByText('Informe um e-mail válido')).toBeInTheDocument()
    expect(campo()).toHaveAttribute('aria-invalid', 'true')
    expect(f).not.toHaveBeenCalled()
  })

  // Cenário "Sem consentimento"
  it('sem consentimento o botão "Me lembrar" fica desabilitado', async () => {
    const f = setup()
    expect(botao()).toBeDisabled()
    await userEvent.type(campo(), 'maria@exemplo.com')
    expect(botao()).toBeDisabled()
    await userEvent.click(consentimento())
    expect(botao()).toBeEnabled()
    await userEvent.click(consentimento())
    expect(botao()).toBeDisabled()
    expect(f).not.toHaveBeenCalled()
  })

  it('o consentimento do lembrete é separado do envio da fatura e explica a exclusão do e-mail', () => {
    setup()
    expect(consentimento()).not.toBeChecked()
    expect(screen.getByText(/separado do envio da fatura/)).toBeInTheDocument()
    expect(screen.getByText(/e-mail é apagado depois do aviso/i)).toBeInTheDocument()
  })

  it('mostra o erro da API sem perder o que foi digitado', async () => {
    setup(() => json({ error: { code: 'reminders_unavailable', message: 'O lembrete por e-mail não está disponível agora' } }, 503))
    await userEvent.click(consentimento())
    await userEvent.type(campo(), 'maria@exemplo.com')
    await userEvent.click(botao())
    expect(await screen.findByRole('alert')).toHaveTextContent('O lembrete por e-mail não está disponível agora')
    expect(campo()).toHaveValue('maria@exemplo.com')
    expect(botao()).toBeEnabled()
  })
})
