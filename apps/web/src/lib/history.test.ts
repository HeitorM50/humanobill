import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { analysis, energyAnalysis } from '../test/fixtures'
import type { Analysis } from '../types/Analysis'
import {
  DISPUTE_DEADLINE,
  ENERGY_VISIT_DAYS,
  ESCALATION,
  HISTORY_KEY,
  HISTORY_MAX,
  clearHistory,
  disputeDeadline,
  findRises,
  historyAvailable,
  markExpired,
  readHistory,
  recordAnalysis,
  recordDispute,
  removeItem,
  type HistoryItem,
} from './history'

/** Meio-dia no fuso local: evita que "virada de dia" mude o resultado conforme o fuso da máquina. */
const day = (month: number, d: number) => new Date(2026, month - 1, d, 12, 0, 0)

function bill(id: string, patch: Partial<Analysis['bill']> = {}, rest: Partial<Analysis> = {}): Analysis {
  return { ...analysis, id, ...rest, bill: { ...analysis.bill, ...patch } }
}

function item(patch: Partial<HistoryItem> = {}): HistoryItem {
  return {
    id: 'x1',
    provider: 'Vivaz Telecom',
    kind: 'telecom',
    month: '2026-10',
    totalCents: 10000,
    recoverableCents: 0,
    seenAt: day(11, 1).toISOString(),
    expiresAt: null,
    expired: false,
    dispute: null,
    ...patch,
  }
}

beforeEach(() => {
  window.localStorage.clear()
  vi.useFakeTimers()
  vi.setSystemTime(day(11, 20))
})
afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

describe('histórico: salvar, remover e apagar', () => {
  it('começa vazio', () => {
    expect(readHistory()).toEqual([])
  })

  it('guarda só os campos da conta, sem nome, CPF ou endereço', () => {
    recordAnalysis({ ...analysis, recoverable_cents: 1290 })
    const [saved] = readHistory()
    expect(saved).toEqual({
      id: 'a1',
      provider: 'Vivaz Telecom',
      kind: 'telecom',
      month: '2026-10',
      totalCents: 12980,
      recoverableCents: 1290,
      seenAt: day(11, 20).toISOString(),
      expiresAt: '2026-12-01T10:00:00.000Z',
      expired: false,
      dispute: null,
    })
    const raw = window.localStorage.getItem(HISTORY_KEY)!
    expect(raw).not.toContain('Clube Vivaz')
    expect(raw).not.toContain('plan_name')
  })

  it('abrir de novo atualiza a conta e a leva para o topo, sem duplicar', () => {
    recordAnalysis(bill('a'))
    vi.setSystemTime(day(11, 21))
    recordAnalysis(bill('b'))
    vi.setSystemTime(day(11, 22))
    recordAnalysis({ ...bill('a'), recoverable_cents: 500 })
    const items = readHistory()
    expect(items.map((i) => i.id)).toEqual(['a', 'b'])
    expect(items[0].recoverableCents).toBe(500)
    expect(items[0].seenAt).toBe(day(11, 22).toISOString())
  })

  it('abrir de novo não perde o protocolo já guardado', () => {
    recordAnalysis(bill('a'))
    recordDispute(bill('a'), '2026-123')
    recordAnalysis(bill('a'))
    expect(readHistory()[0].dispute).toEqual({ protocol: '2026-123', at: day(11, 20).toISOString() })
  })

  it('remove um item', () => {
    recordAnalysis(bill('a'))
    recordAnalysis(bill('b'))
    removeItem('a')
    expect(readHistory().map((i) => i.id)).toEqual(['b'])
  })

  it('apaga tudo', () => {
    recordAnalysis(bill('a'))
    recordAnalysis(bill('b'))
    clearHistory()
    expect(readHistory()).toEqual([])
    expect(window.localStorage.getItem(HISTORY_KEY)).toBeNull()
  })

  it('guarda no máximo 50 itens e descarta os mais antigos', () => {
    for (let i = 0; i < HISTORY_MAX + 5; i++) {
      vi.setSystemTime(new Date(2026, 10, 1, 12, 0, i))
      recordAnalysis(bill(`id${i}`))
    }
    const items = readHistory()
    expect(items).toHaveLength(HISTORY_MAX)
    expect(items[0].id).toBe(`id${HISTORY_MAX + 4}`)
    expect(items.some((i) => i.id === 'id0')).toBe(false)
  })

  it('marca como expirado, sem apagar o protocolo', () => {
    recordAnalysis(bill('a'))
    recordDispute(bill('a'), '123')
    markExpired('a')
    const [saved] = readHistory()
    expect(saved.expired).toBe(true)
    expect(saved.dispute?.protocol).toBe('123')
  })

  it('ignora ids que não estão na lista ao marcar como expirado', () => {
    markExpired('nao-existe')
    expect(readHistory()).toEqual([])
  })
})

describe('histórico: protocolo da contestação', () => {
  it('guarda protocolo e data, e cria o item se ainda não existir', () => {
    recordDispute(bill('a'), ' 2026-0042 ')
    expect(readHistory()[0].dispute).toEqual({ protocol: '2026-0042', at: day(11, 20).toISOString() })
  })

  it('repetir o mesmo protocolo mantém a data original', () => {
    recordDispute(bill('a'), '123')
    vi.setSystemTime(day(11, 25))
    recordDispute(bill('a'), '123')
    expect(readHistory()[0].dispute?.at).toBe(day(11, 20).toISOString())
  })

  it('um protocolo novo troca o protocolo e a data', () => {
    recordDispute(bill('a'), '123')
    vi.setSystemTime(day(11, 25))
    recordDispute(bill('a'), '456')
    expect(readHistory()[0].dispute).toEqual({ protocol: '456', at: day(11, 25).toISOString() })
  })

  it.each(['', '   ', 'abc', '12 34', 'x'.repeat(31), '1'.repeat(31)])('não guarda protocolo inválido (%j)', (p) => {
    recordDispute(bill('a'), p)
    expect(readHistory()).toEqual([])
  })
})

describe('histórico: dados corrompidos', () => {
  it('ignora JSON inválido', () => {
    window.localStorage.setItem(HISTORY_KEY, '{nao e json')
    expect(readHistory()).toEqual([])
  })

  it('ignora versão desconhecida e itens malformados', () => {
    window.localStorage.setItem(HISTORY_KEY, JSON.stringify({ v: 2, items: [item()] }))
    expect(readHistory()).toEqual([])
    window.localStorage.setItem(
      HISTORY_KEY,
      JSON.stringify({
        v: 1,
        items: [item({ id: 'bom' }), { id: 'ruim' }, item({ id: 'mes-ruim', month: '2026-13' }), item({ id: 'bom' }), null, 7],
      }),
    )
    expect(readHistory().map((i) => i.id)).toEqual(['bom'])
  })

  it('descarta protocolo guardado fora do formato', () => {
    window.localStorage.setItem(
      HISTORY_KEY,
      JSON.stringify({ v: 1, items: [item({ dispute: { protocol: '<script>', at: day(11, 1).toISOString() } })] }),
    )
    expect(readHistory()).toHaveLength(1)
    expect(readHistory()[0].dispute).toBeNull()
  })
})

describe('histórico: localStorage indisponível', () => {
  it('não quebra e a feature fica indisponível', () => {
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new DOMException('bloqueado', 'SecurityError')
    })
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new DOMException('bloqueado', 'SecurityError')
    })
    vi.spyOn(Storage.prototype, 'removeItem').mockImplementation(() => {
      throw new DOMException('bloqueado', 'SecurityError')
    })
    expect(historyAvailable()).toBe(false)
    expect(readHistory()).toEqual([])
    expect(() => recordAnalysis(bill('a'))).not.toThrow()
    expect(() => recordDispute(bill('a'), '123')).not.toThrow()
    expect(() => markExpired('a')).not.toThrow()
    expect(() => removeItem('a')).not.toThrow()
    expect(() => clearHistory()).not.toThrow()
    expect(readHistory()).toEqual([])
  })

  it('cota cheia: a escrita falha em silêncio', () => {
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new DOMException('cheio', 'QuotaExceededError')
    })
    expect(() => recordAnalysis(bill('a'))).not.toThrow()
  })

  it('fica disponível quando o navegador deixa guardar', () => {
    expect(historyAvailable()).toBe(true)
    // O teste de disponibilidade não deixa lixo no armazenamento.
    expect(window.localStorage.length).toBe(0)
  })
})

describe('prazo da empresa: telecom, 30 dias corridos', () => {
  const contested = item({ dispute: { protocol: '123', at: day(11, 1).toISOString() } })

  it('telecom tem 30 dias corridos; energia, 5 dias úteis (como em dispute.rs)', () => {
    expect(DISPUTE_DEADLINE.telecom).toEqual({ days: 30, business: false })
    expect(DISPUTE_DEADLINE.energy).toEqual({ days: 5, business: true })
    expect(ENERGY_VISIT_DAYS).toBe(10)
  })

  it('telecom conta fim de semana', () => {
    // 01/11/2026 é domingo e 01/12/2026 é terça: o prazo não pula nada.
    expect(disputeDeadline(contested, day(11, 1))).toMatchObject({ business: false, limitDays: 30 })
  })

  it('sem contestação não há prazo', () => {
    expect(disputeDeadline(item(), day(11, 20))).toBeNull()
  })

  it('antes: conta quantos dias faltam', () => {
    expect(disputeDeadline(contested, day(11, 1))).toMatchObject({ status: 'aberto', daysLeft: 30 })
    expect(disputeDeadline(contested, day(11, 20))).toMatchObject({ status: 'aberto', daysLeft: 11 })
    expect(disputeDeadline(contested, day(11, 30))).toMatchObject({ status: 'aberto', daysLeft: 1 })
  })

  it('no dia: hoje é o último dia, ainda não venceu', () => {
    expect(disputeDeadline(contested, day(12, 1))).toMatchObject({ status: 'hoje', daysLeft: 0 })
  })

  it('vencido: a partir do dia seguinte', () => {
    expect(disputeDeadline(contested, day(12, 2))).toMatchObject({ status: 'vencido', daysLate: 1 })
    expect(disputeDeadline(contested, day(12, 25))).toMatchObject({ status: 'vencido', daysLate: 24 })
  })

  it('usa a data fixa do relógio simulado quando não recebe `now`', () => {
    vi.setSystemTime(day(12, 5))
    expect(disputeDeadline(contested)).toMatchObject({ status: 'vencido', daysLate: 4 })
  })

  it('conta por dia do calendário, não por horas corridas', () => {
    const lateNight = item({ dispute: { protocol: '1', at: new Date(2026, 10, 1, 23, 50).toISOString() } })
    expect(disputeDeadline(lateNight, new Date(2026, 10, 2, 0, 10))).toMatchObject({ status: 'aberto', daysLeft: 29 })
  })

  it('a data limite vem no formato ISO local', () => {
    expect(disputeDeadline(contested, day(11, 20))?.deadline).toBe('2026-12-01')
  })
})

describe('próximo passo quando o prazo vence', () => {
  it('telecom: Anatel e consumidor.gov.br, com links oficiais', () => {
    const steps = ESCALATION.telecom.steps
    expect(steps.map((s) => s.title)).toEqual(['Anatel', 'consumidor.gov.br'])
    expect(steps[0].url).toBe('https://www.gov.br/anatel/pt-br/consumidor/reclamacoes')
    expect(steps[1].url).toBe('https://www.consumidor.gov.br')
  })

  it('energia: ouvidoria da distribuidora e ANEEL', () => {
    const steps = ESCALATION.energy.steps
    expect(steps.map((s) => s.title)).toEqual(['Ouvidoria da distribuidora', 'ANEEL'])
    expect(steps[0].url).toBeUndefined()
    expect(steps[1].url).toBe('https://www.gov.br/aneel/pt-br/consumidores/como-registrar-reclamacao')
  })

  it('todo link é https', () => {
    for (const k of ['telecom', 'energy'] as const)
      for (const s of ESCALATION[k].steps) if (s.url) expect(s.url.startsWith('https://')).toBe(true)
  })
})

describe('alerta de aumento (só com os totais guardados)', () => {
  const mk = (id: string, month: string, totalCents: number, patch: Partial<HistoryItem> = {}) =>
    item({ id, month, totalCents, ...patch })

  it('avisa quando sobe 15% ou mais', () => {
    const rises = findRises([mk('set', '2026-09', 10000), mk('out', '2026-10', 11500)])
    expect(rises).toEqual([
      { provider: 'Vivaz Telecom', kind: 'telecom', fromId: 'set', toId: 'out', fromMonth: '2026-09', toMonth: '2026-10', pct: 15 },
    ])
  })

  it('não avisa abaixo de 15%', () => {
    expect(findRises([mk('set', '2026-09', 10000), mk('out', '2026-10', 11499)])).toEqual([])
  })

  it('não avisa quando a conta cai ou fica igual', () => {
    expect(findRises([mk('set', '2026-09', 10000), mk('out', '2026-10', 8000)])).toEqual([])
    expect(findRises([mk('set', '2026-09', 10000), mk('out', '2026-10', 10000)])).toEqual([])
  })

  it('arredonda o percentual para exibir', () => {
    const [r] = findRises([mk('set', '2026-09', 10000), mk('out', '2026-10', 12349)])
    expect(r.pct).toBe(23)
  })

  it('não mistura empresas nem tipos de conta', () => {
    expect(findRises([mk('a', '2026-09', 10000), mk('b', '2026-10', 20000, { provider: 'Outra Telecom' })])).toEqual([])
    expect(findRises([mk('a', '2026-09', 10000), mk('b', '2026-10', 20000, { kind: 'energy' })])).toEqual([])
  })

  it('ignora diferença de maiúsculas, acento e espaços no nome da empresa', () => {
    const rises = findRises([mk('a', '2026-09', 10000), mk('b', '2026-10', 20000, { provider: '  VIVAZ   telecom ' })])
    expect(rises).toHaveLength(1)
  })

  it('exige meses diferentes', () => {
    expect(findRises([mk('a', '2026-10', 10000), mk('b', '2026-10', 20000)])).toEqual([])
  })

  it('compara os dois meses mais recentes, qualquer que seja a ordem da lista', () => {
    const rises = findRises([
      mk('out', '2026-10', 11500),
      mk('ago', '2026-08', 5000),
      mk('set', '2026-09', 10000),
    ])
    expect(rises).toHaveLength(1)
    expect(rises[0]).toMatchObject({ fromId: 'set', toId: 'out' })
  })

  it('no mesmo mês, vale a conta vista por último', () => {
    const rises = findRises([
      mk('set', '2026-09', 10000),
      mk('out-velha', '2026-10', 10100, { seenAt: day(11, 1).toISOString() }),
      mk('out-nova', '2026-10', 12000, { seenAt: day(11, 10).toISOString() }),
    ])
    expect(rises[0]).toMatchObject({ toId: 'out-nova', pct: 20 })
  })

  it('conta anterior de R$ 0 não gera aviso', () => {
    expect(findRises([mk('a', '2026-09', 0), mk('b', '2026-10', 5000)])).toEqual([])
  })

  it('funciona com energia', () => {
    const e = (id: string, month: string, cents: number) =>
      item({ id, month, totalCents: cents, kind: 'energy', provider: energyAnalysis.bill.provider_name })
    expect(findRises([e('a', '2026-09', 20000), e('b', '2026-10', 30000)])[0]).toMatchObject({ kind: 'energy', pct: 50 })
  })
})

describe('prazo da empresa: energia, 5 dias úteis', () => {
  // 06/11/2026 é sexta-feira. Os 5 dias úteis são seg 09, ter 10, qua 11, qui 12 e sex 13.
  const at = (m: number, d: number) => item({ kind: 'energy', provider: 'Luzia Energia', dispute: { protocol: '9', at: day(m, d).toISOString() } })

  it('sexta + 5 dias úteis vence na sexta seguinte, pulando o fim de semana', () => {
    expect(disputeDeadline(at(11, 6), day(11, 6))).toMatchObject({ status: 'aberto', daysLeft: 5, deadline: '2026-11-13', business: true, limitDays: 5 })
  })

  it('antes: cada dia útil que passa tira 1; o fim de semana não conta', () => {
    expect(disputeDeadline(at(11, 6), day(11, 7))).toMatchObject({ status: 'aberto', daysLeft: 5 }) // sábado
    expect(disputeDeadline(at(11, 6), day(11, 8))).toMatchObject({ status: 'aberto', daysLeft: 5 }) // domingo
    expect(disputeDeadline(at(11, 6), day(11, 9))).toMatchObject({ status: 'aberto', daysLeft: 4 })
    expect(disputeDeadline(at(11, 6), day(11, 12))).toMatchObject({ status: 'aberto', daysLeft: 1 })
  })

  it('no dia: sexta, último dia útil, ainda não venceu', () => {
    expect(disputeDeadline(at(11, 6), day(11, 13))).toMatchObject({ status: 'hoje', daysLeft: 0 })
  })

  it('vencido no dia seguinte', () => {
    expect(disputeDeadline(at(11, 6), day(11, 14))).toMatchObject({ status: 'vencido', daysLate: 1 })
    expect(disputeDeadline(at(11, 6), day(11, 20))).toMatchObject({ status: 'vencido', daysLate: 7 })
  })

  it('contestar na quinta cruza o fim de semana: vence na quinta seguinte', () => {
    expect(disputeDeadline(at(11, 12), day(11, 12))).toMatchObject({ status: 'aberto', daysLeft: 5, deadline: '2026-11-19' })
    expect(disputeDeadline(at(11, 12), day(11, 14))).toMatchObject({ status: 'aberto', daysLeft: 4 })
    expect(disputeDeadline(at(11, 12), day(11, 19))).toMatchObject({ status: 'hoje' })
    expect(disputeDeadline(at(11, 12), day(11, 20))).toMatchObject({ status: 'vencido', daysLate: 1 })
  })

  it('contestar no sábado: a contagem começa na segunda', () => {
    expect(disputeDeadline(at(11, 7), day(11, 7))).toMatchObject({ status: 'aberto', daysLeft: 5, deadline: '2026-11-13' })
  })

  it('feriado não é descontado: só sábado e domingo ficam de fora', () => {
    // 12/10/2026 (segunda) é feriado nacional; a conta ignora isso de propósito.
    expect(disputeDeadline(at(10, 9), day(10, 9))).toMatchObject({ deadline: '2026-10-16' })
  })
})
