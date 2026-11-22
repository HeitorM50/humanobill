import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { HISTORY_KEY, markExpired, readHistory, recordAnalysis, recordDispute } from '../lib/history'
import { analysis, energyAnalysis } from '../test/fixtures'
import type { Analysis } from '../types/Analysis'
import MyBills from './MyBills'

/** Meio-dia no fuso local, para a virada de dia não depender da máquina. */
const day = (month: number, d: number) => new Date(2026, month - 1, d, 12, 0, 0)

const A = '11111111-1111-4111-8111-111111111111'
const B = '22222222-2222-4222-8222-222222222222'

function bill(id: string, patch: Partial<Analysis['bill']> = {}, rest: Partial<Analysis> = {}): Analysis {
  return { ...analysis, id, expires_at: '2027-01-01T10:00:00Z', ...rest, bill: { ...analysis.bill, ...patch } }
}

function mount() {
  return render(
    <MemoryRouter initialEntries={['/minhas-contas']}>
      <Routes>
        <Route path="minhas-contas" element={<MyBills />} />
        <Route path="*" element={<p>outra página</p>} />
      </Routes>
    </MemoryRouter>,
  )
}

beforeEach(() => {
  window.localStorage.clear()
  // Só o relógio: os timers de verdade continuam, para o userEvent funcionar.
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(day(11, 20))
})
afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

describe('Minhas contas: lista', () => {
  it('mostra o aviso de privacidade e o estado vazio', () => {
    mount()
    expect(screen.getByRole('heading', { level: 1, name: 'Minhas contas neste aparelho' })).toBeInTheDocument()
    expect(screen.getByText('Fica só neste aparelho. Apagar o histórico do navegador apaga esta lista.')).toBeInTheDocument()
    expect(screen.getByText('Nenhuma conta por aqui ainda.')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Apagar tudo' })).not.toBeInTheDocument()
  })

  it('lista a conta com empresa, mês, total e valor recuperável', () => {
    recordAnalysis({ ...bill(A), recoverable_cents: 1290 })
    mount()
    const card = screen.getByRole('listitem')
    expect(card).toHaveTextContent('Celular/Internet · outubro de 2026')
    expect(card).toHaveTextContent('R$ 129,80')
    expect(card).toHaveTextContent('Dá para recuperar R$ 12,90')
    expect(card).toHaveTextContent('Visto em 20/11/2026')
    expect(screen.getByRole('link', { name: 'Abrir o resultado de Vivaz Telecom, outubro de 2026' })).toHaveAttribute('href', `/r/${A}`)
  })

  it('remove só um item', async () => {
    recordAnalysis(bill(A))
    recordAnalysis(bill(B, { provider_name: 'Luzia Energia', kind: 'energy' }))
    mount()
    await userEvent.click(screen.getByRole('button', { name: 'Remover Vivaz Telecom, outubro de 2026, da lista' }))
    expect(screen.queryByRole('heading', { name: 'Vivaz Telecom' })).not.toBeInTheDocument()
    expect(screen.getByRole('heading', { name: 'Luzia Energia' })).toBeInTheDocument()
    expect(readHistory().map((i) => i.id)).toEqual([B])
    expect(screen.getByRole('status')).toHaveTextContent('removida da lista')
  })

  it('apaga tudo só depois de confirmar', async () => {
    recordAnalysis(bill(A))
    recordAnalysis(bill(B))
    mount()
    await userEvent.click(screen.getByRole('button', { name: 'Apagar tudo' }))
    // Ainda não apagou: pediu confirmação.
    expect(readHistory()).toHaveLength(2)
    await userEvent.click(screen.getByRole('button', { name: 'Cancelar' }))
    expect(readHistory()).toHaveLength(2)

    await userEvent.click(screen.getByRole('button', { name: 'Apagar tudo' }))
    await userEvent.click(screen.getByRole('button', { name: 'Sim, apagar tudo' }))
    expect(readHistory()).toEqual([])
    expect(window.localStorage.getItem(HISTORY_KEY)).toBeNull()
    expect(screen.getByText('Nenhuma conta por aqui ainda.')).toBeInTheDocument()
  })

  it('resultado expirado aparece como expirado e não tem link para abrir', () => {
    recordAnalysis(bill(A))
    markExpired(A)
    mount()
    expect(screen.getByText(/Resultado expirado/)).toBeInTheDocument()
    expect(screen.queryByRole('link', { name: /Abrir o resultado/ })).not.toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'Analisar de novo' })).toHaveAttribute('href', '/analisar')
  })

  it('também marca como expirado pela data de validade guardada', () => {
    recordAnalysis(bill(A, {}, { expires_at: '2026-11-10T10:00:00Z' }))
    mount()
    expect(screen.getByText(/Resultado expirado/)).toBeInTheDocument()
  })

  it('a conta expirada ainda mostra o protocolo e o prazo', () => {
    recordDispute(bill(A), '2026-0042', day(11, 1))
    markExpired(A)
    mount()
    expect(screen.getByText('2026-0042')).toBeInTheDocument()
    expect(screen.getByText(/Faltam 11 dias do prazo de 30 dias da empresa/)).toBeInTheDocument()
  })
})

describe('Minhas contas: prazo da contestação', () => {
  const seed = (kind: 'telecom' | 'energy' = 'telecom', at: Date = day(11, 1)) => {
    const base = kind === 'energy' ? { ...energyAnalysis, expires_at: '2027-01-01T10:00:00Z' } : bill(A)
    recordDispute({ ...base, id: A }, '2026-0042', at)
  }

  it('antes: conta quantos dias faltam, com a data limite', () => {
    seed()
    mount()
    expect(screen.getByText('Faltam 11 dias do prazo de 30 dias da empresa (até 01/12/2026).')).toBeInTheDocument()
    expect(screen.getByText(/Contestação feita em 01\/11\/2026/)).toBeInTheDocument()
    expect(screen.getByText('2026-0042')).toBeInTheDocument()
    expect(screen.queryByText('Prazo vencido: próximo passo')).not.toBeInTheDocument()
  })

  it('usa o singular quando falta 1 dia', () => {
    seed()
    vi.setSystemTime(day(11, 30))
    mount()
    expect(screen.getByText('Falta 1 dia do prazo de 30 dias da empresa (até 01/12/2026).')).toBeInTheDocument()
  })

  it('no dia: avisa que é o último dia e ainda não escala', () => {
    seed()
    vi.setSystemTime(day(12, 1))
    mount()
    expect(screen.getByText(/Hoje é o último dia do prazo de 30 dias/)).toBeInTheDocument()
    expect(screen.queryByText('Prazo vencido: próximo passo')).not.toBeInTheDocument()
  })

  it('vencido (telecom): mostra Anatel e consumidor.gov.br com os links oficiais', () => {
    seed()
    vi.setSystemTime(day(12, 2))
    mount()
    expect(screen.getByText('Prazo vencido: próximo passo')).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'Anatel (abre em nova aba)' })).toHaveAttribute('href', 'https://www.gov.br/anatel/pt-br/consumidor/reclamacoes')
    expect(screen.getByRole('link', { name: 'consumidor.gov.br (abre em nova aba)' })).toHaveAttribute('href', 'https://www.consumidor.gov.br')
    expect(screen.queryByText(/Faltam/)).not.toBeInTheDocument()
  })

  it('vencido (energia): ouvidoria da distribuidora (10 dias úteis) e ANEEL', () => {
    seed('energy', day(11, 6))
    vi.setSystemTime(day(11, 14)) // sábado: o prazo acabou na sexta 13
    mount()
    expect(screen.getByText('Prazo vencido: próximo passo')).toBeInTheDocument()
    expect(screen.getByText('Ouvidoria da distribuidora')).toBeInTheDocument()
    expect(screen.getByText(/Ela responde em até 10 dias úteis/)).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'ANEEL (abre em nova aba)' })).toHaveAttribute(
      'href',
      'https://www.gov.br/aneel/pt-br/consumidores/como-registrar-reclamacao',
    )
    expect(screen.queryByRole('link', { name: /Anatel/ })).not.toBeInTheDocument()
    expect(screen.queryByText(/Faltam|Falta /)).not.toBeInTheDocument()
  })

  it('energia: conta 5 dias úteis, avisa que feriado não entra e cita os 10 com visita técnica', () => {
    seed('energy', day(11, 6))
    vi.setSystemTime(day(11, 9)) // segunda: depois do fim de semana
    mount()
    expect(screen.getByText('Faltam 4 dias úteis do prazo de 5 dias úteis da distribuidora (até 13/11/2026).')).toBeInTheDocument()
    expect(screen.getByText(/Conta só dias de semana, sem descontar feriados/)).toBeInTheDocument()
    expect(screen.getByText(/Com visita técnica, a distribuidora tem 10 dias úteis/)).toBeInTheDocument()
  })

  it('energia, no dia: último dia útil e ainda sem escalada', () => {
    seed('energy', day(11, 6))
    vi.setSystemTime(day(11, 13))
    mount()
    expect(screen.getByText(/Hoje é o último dia do prazo de 5 dias úteis da distribuidora/)).toBeInTheDocument()
    expect(screen.queryByText('Prazo vencido: próximo passo')).not.toBeInTheDocument()
  })

  it('energia, singular: falta 1 dia útil', () => {
    seed('energy', day(11, 6))
    vi.setSystemTime(day(11, 12))
    mount()
    expect(screen.getByText('Falta 1 dia útil do prazo de 5 dias úteis da distribuidora (até 13/11/2026).')).toBeInTheDocument()
  })

  it('telecom não mostra a nota de dias úteis', () => {
    seed()
    mount()
    expect(screen.queryByText(/dias úteis/)).not.toBeInTheDocument()
  })

  it('links externos abrem em nova aba sem vazar a origem', () => {
    seed()
    vi.setSystemTime(day(12, 2))
    mount()
    for (const a of screen.getAllByRole('link', { name: /abre em nova aba/ })) {
      expect(a).toHaveAttribute('target', '_blank')
      expect(a).toHaveAttribute('rel', 'noopener noreferrer')
    }
  })

  it('conta sem contestação não mostra prazo', () => {
    recordAnalysis(bill(A))
    mount()
    expect(screen.queryByText(/prazo de 30 dias/)).not.toBeInTheDocument()
  })
})

describe('Minhas contas: alerta de aumento', () => {
  const two = (afterCents: number) => {
    recordAnalysis(bill(A, { reference_month: '2026-09', total_cents: 10000 }))
    recordAnalysis(bill(B, { reference_month: '2026-10', total_cents: afterCents }))
  }

  it('avisa com 15% ou mais e leva para a comparação', () => {
    two(11500)
    mount()
    expect(screen.getByText(/Sua conta de Vivaz Telecom subiu/)).toHaveTextContent(
      'Sua conta de Vivaz Telecom subiu 15% de setembro de 2026 para outubro de 2026.',
    )
    expect(screen.getByRole('link', { name: 'Ver o que mudou' })).toHaveAttribute('href', `/comparar?a=${A}&b=${B}`)
  })

  it('não avisa com menos de 15%', () => {
    two(11400)
    mount()
    expect(screen.queryByText(/subiu/)).not.toBeInTheDocument()
    expect(screen.queryByRole('heading', { name: 'Aumento na conta' })).not.toBeInTheDocument()
  })

  it('sem link de comparação quando um dos resultados expirou', () => {
    two(15000)
    markExpired(A)
    mount()
    expect(screen.getByText(/Sua conta de Vivaz Telecom subiu/)).toHaveTextContent('subiu 50%')
    expect(screen.queryByRole('link', { name: 'Ver o que mudou' })).not.toBeInTheDocument()
  })

  it('não compara empresas diferentes', () => {
    recordAnalysis(bill(A, { reference_month: '2026-09', total_cents: 10000 }))
    recordAnalysis(bill(B, { reference_month: '2026-10', total_cents: 30000, provider_name: 'Vivaz Fibra' }))
    mount()
    expect(screen.queryByText(/subiu/)).not.toBeInTheDocument()
  })
})

describe('Minhas contas: sem localStorage', () => {
  it('avisa em vez de quebrar', () => {
    for (const m of ['getItem', 'setItem', 'removeItem'] as const)
      vi.spyOn(Storage.prototype, m).mockImplementation(() => {
        throw new DOMException('bloqueado', 'SecurityError')
      })
    mount()
    expect(screen.getByText('Este navegador não deixa guardar a lista')).toBeInTheDocument()
    expect(screen.queryByText('Nenhuma conta por aqui ainda.')).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Apagar tudo' })).not.toBeInTheDocument()
  })
})
