import { useSyncExternalStore } from 'react'
import type { Analysis } from '../types/Analysis'
import type { BillKind } from '../types/BillKind'

/**
 * "Minhas contas neste aparelho" (Acompanhar).
 *
 * Não há login: a lista vive só no `localStorage` do navegador. Nada daqui vai para o servidor e
 * nada identifica a pessoa (sem nome, CPF, endereço ou e-mail). Se o armazenamento estiver
 * bloqueado ou cheio, tudo vira "não faz nada": a feature some, mas a página não quebra.
 */

export const HISTORY_KEY = 'humanobill:minhas-contas'
const PROBE_KEY = 'humanobill:minhas-contas:teste'
const VERSION = 1

/** Quantas contas ficam na lista. As mais antigas saem primeiro. */
export const HISTORY_MAX = 50
/** A partir de quanto (em %) um aumento merece aviso. */
export const RISE_THRESHOLD_PCT = 15

const PROVIDER_MAX = 80
// Mesma regra do servidor e do formulário: só dígitos e hífen, até 30 caracteres.
const PROTOCOL_RE = /^[0-9-]{1,30}$/
const MONTH_RE = /^\d{4}-(0[1-9]|1[0-2])$/

export type HistoryDispute = { protocol: string; at: string }

export type HistoryItem = {
  id: string
  provider: string
  kind: BillKind
  /** "YYYY-MM". */
  month: string
  totalCents: number
  recoverableCents: number
  /** Quando a pessoa abriu este resultado pela última vez (ISO). */
  seenAt: string
  /** Quando o resultado some do servidor (ISO), se a API informou. */
  expiresAt: string | null
  /** A API respondeu "não existe mais" (404 ou 410). */
  expired: boolean
  dispute: HistoryDispute | null
}

/* ---------- Leitura e escrita no localStorage ---------- */

const listeners = new Set<() => void>()
const notify = () => listeners.forEach((l) => l())

const EMPTY: HistoryItem[] = []
let cache: { raw: string | null; items: HistoryItem[] } = { raw: null, items: EMPTY }

function readRaw(): string | null {
  try {
    return window.localStorage.getItem(HISTORY_KEY)
  } catch {
    return null
  }
}

const isDate = (v: unknown): v is string => typeof v === 'string' && !Number.isNaN(Date.parse(v))
const isCents = (v: unknown): v is number => typeof v === 'number' && Number.isSafeInteger(v) && v >= 0

/** Confere um item lido do disco. O que não bate com o formato é descartado, nunca consertado. */
function sanitize(v: unknown): HistoryItem | null {
  if (typeof v !== 'object' || v === null) return null
  const o = v as Record<string, unknown>
  if (typeof o.id !== 'string' || o.id === '' || o.id.length > 100) return null
  if (typeof o.provider !== 'string' || o.provider === '' || o.provider.length > PROVIDER_MAX) return null
  if (o.kind !== 'telecom' && o.kind !== 'energy') return null
  if (typeof o.month !== 'string' || !MONTH_RE.test(o.month)) return null
  if (!isCents(o.totalCents) || !isCents(o.recoverableCents) || !isDate(o.seenAt)) return null
  const d = o.dispute as Record<string, unknown> | null | undefined
  const dispute = d && typeof d.protocol === 'string' && PROTOCOL_RE.test(d.protocol) && isDate(d.at) ? { protocol: d.protocol, at: d.at } : null
  return {
    id: o.id,
    provider: o.provider,
    kind: o.kind,
    month: o.month,
    totalCents: o.totalCents,
    recoverableCents: o.recoverableCents,
    seenAt: o.seenAt,
    expiresAt: isDate(o.expiresAt) ? o.expiresAt : null,
    expired: o.expired === true,
    dispute,
  }
}

function parse(raw: string | null): HistoryItem[] {
  if (!raw) return EMPTY
  try {
    const data = JSON.parse(raw) as { v?: unknown; items?: unknown }
    if (data?.v !== VERSION || !Array.isArray(data.items)) return EMPTY
    const seen = new Set<string>()
    const items: HistoryItem[] = []
    for (const candidate of data.items) {
      const item = sanitize(candidate)
      if (!item || seen.has(item.id)) continue
      seen.add(item.id)
      items.push(item)
    }
    return items.slice(0, HISTORY_MAX)
  } catch {
    return EMPTY
  }
}

/** A lista atual, da mais recente para a mais antiga. O resultado só muda de identidade quando o conteúdo muda. */
export function readHistory(): HistoryItem[] {
  const raw = readRaw()
  if (raw !== cache.raw) cache = { raw, items: parse(raw) }
  return cache.items
}

function write(items: HistoryItem[]) {
  try {
    if (items.length === 0) window.localStorage.removeItem(HISTORY_KEY)
    else window.localStorage.setItem(HISTORY_KEY, JSON.stringify({ v: VERSION, items: items.slice(0, HISTORY_MAX) }))
  } catch {
    /* sem armazenamento (modo privado, cota cheia): a lista simplesmente não guarda */
  }
  notify()
}

/** O navegador deixa guardar? Quando não, o app esconde a feature em vez de prometer o que não cumpre. */
export function historyAvailable(): boolean {
  try {
    window.localStorage.setItem(PROBE_KEY, '1')
    window.localStorage.removeItem(PROBE_KEY)
    return true
  } catch {
    return false
  }
}

/* ---------- Mudanças na lista ---------- */

function toItem(a: Analysis, now: Date, previous?: HistoryItem): HistoryItem {
  const expires = new Date(a.expires_at)
  return {
    id: a.id,
    provider: a.bill.provider_name.trim().slice(0, PROVIDER_MAX) || 'Empresa',
    kind: a.bill.kind,
    month: a.bill.reference_month,
    totalCents: Math.max(0, Math.round(a.bill.total_cents)),
    recoverableCents: Math.max(0, Math.round(a.recoverable_cents)),
    seenAt: now.toISOString(),
    expiresAt: Number.isNaN(expires.getTime()) ? null : expires.toISOString(),
    // Se acabamos de abrir o resultado, ele existe.
    expired: false,
    dispute: previous?.dispute ?? null,
  }
}

/** Põe (ou atualiza) a conta no topo da lista. Chame ao abrir um resultado. */
function upsert(a: Analysis, now: Date, change?: (item: HistoryItem) => HistoryItem) {
  const items = readHistory()
  const previous = items.find((i) => i.id === a.id)
  let next = toItem(a, now, previous)
  if (change) next = change(next)
  write([next, ...items.filter((i) => i.id !== a.id)].slice(0, HISTORY_MAX))
}

export function recordAnalysis(a: Analysis, now: Date = new Date()) {
  if (!MONTH_RE.test(a.bill.reference_month)) return
  upsert(a, now)
}

/** Guarda o protocolo e a data em que a contestação foi gerada. Protocolo fora do formato é ignorado. */
export function recordDispute(a: Analysis, protocol: string, now: Date = new Date()) {
  const p = protocol.trim()
  if (!PROTOCOL_RE.test(p) || !MONTH_RE.test(a.bill.reference_month)) return
  const previous = readHistory().find((i) => i.id === a.id)
  upsert(a, now, (item) => ({
    ...item,
    // O mesmo protocolo não "renova" o prazo: a contagem vale desde a primeira vez.
    dispute: previous?.dispute?.protocol === p ? previous.dispute : { protocol: p, at: now.toISOString() },
  }))
}

/** A API disse que o resultado não existe mais. Mantém o protocolo: a escalada não depende do resultado. */
export function markExpired(id: string) {
  const items = readHistory()
  if (!items.some((i) => i.id === id && !i.expired)) return
  write(items.map((i) => (i.id === id ? { ...i, expired: true } : i)))
}

export function removeItem(id: string) {
  const items = readHistory()
  if (!items.some((i) => i.id === id)) return
  write(items.filter((i) => i.id !== id))
}

export function clearHistory() {
  write([])
}

/** O resultado ainda abre? Vale a informação da API, ou a data de validade guardada. */
export function isExpired(item: HistoryItem, now: Date = new Date()): boolean {
  return item.expired || (item.expiresAt !== null && Date.parse(item.expiresAt) <= now.getTime())
}

/* ---------- React ---------- */

function subscribe(onChange: () => void) {
  listeners.add(onChange)
  // Outra aba mexeu na lista.
  const onStorage = (e: StorageEvent) => {
    if (e.key === null || e.key === HISTORY_KEY) onChange()
  }
  window.addEventListener('storage', onStorage)
  return () => {
    listeners.delete(onChange)
    window.removeEventListener('storage', onStorage)
  }
}

export function useHistory(): HistoryItem[] {
  return useSyncExternalStore(subscribe, readHistory, () => EMPTY)
}

/* ---------- Prazo da empresa ---------- */

/**
 * Prazo para a empresa responder, igual ao de `crates/rules/src/dispute.rs`:
 * telecom = 30 dias corridos (a operadora responde);
 * energia = 5 dias úteis para a distribuidora resolver (REN ANEEL 1.000/2021, art. 408).
 * Com visita técnica a distribuidora tem 10 dias úteis; a contagem mostra só os 5.
 */
export const DISPUTE_DEADLINE: Record<BillKind, { days: number; business: boolean }> = {
  telecom: { days: 30, business: false },
  energy: { days: 5, business: true },
}
/** Na energia, com visita técnica, o prazo da distribuidora passa para 10 dias úteis (só citado no texto). */
export const ENERGY_VISIT_DAYS = 10

/** Dia do calendário local como número de dias (sem hora), para contar "dias" e não "24 horas". */
const calendarDay = (d: Date) => Math.floor(Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()) / 86_400_000)
/** O dia 0 (01/01/1970) foi uma quinta-feira. 0 = domingo e 6 = sábado. */
const weekday = (day: number) => (((day + 4) % 7) + 7) % 7
const isWeekend = (day: number) => weekday(day) === 0 || weekday(day) === 6

/** Soma dias úteis: pula sábado e domingo. Feriados não entram na conta. */
function addBusinessDays(day: number, n: number): number {
  let d = day
  for (let left = n; left > 0; ) {
    d += 1
    if (!isWeekend(d)) left -= 1
  }
  return d
}

/** Quantos dias úteis há depois de `from` até `to` (inclusive). */
function businessDaysBetween(from: number, to: number): number {
  let n = 0
  for (let d = from + 1; d <= to; d++) if (!isWeekend(d)) n += 1
  return n
}

export type DisputeDeadline = { business: boolean; limitDays: number; deadline: string } & (
  /** Ainda dentro do prazo. */
  | { status: 'aberto'; daysLeft: number }
  /** Hoje é o último dia. */
  | { status: 'hoje'; daysLeft: 0 }
  /** O prazo passou: hora de escalar. `daysLate` conta dias do calendário. */
  | { status: 'vencido'; daysLate: number }
)

/** Quanto falta (ou quanto passou) do prazo da empresa. `null` se não houve contestação. */
export function disputeDeadline(item: HistoryItem, now: Date = new Date()): DisputeDeadline | null {
  if (!item.dispute) return null
  const { days, business } = DISPUTE_DEADLINE[item.kind]
  const start = calendarDay(new Date(item.dispute.at))
  const limit = business ? addBusinessDays(start, days) : start + days
  const today = calendarDay(now)
  const base = { business, limitDays: days, deadline: new Date(limit * 86_400_000).toISOString().slice(0, 10) }
  if (today > limit) return { ...base, status: 'vencido', daysLate: today - limit }
  if (today === limit) return { ...base, status: 'hoje', daysLeft: 0 }
  return { ...base, status: 'aberto', daysLeft: business ? businessDaysBetween(today, limit) : limit - today }
}

/* ---------- Próximo passo depois do prazo ---------- */

export type EscalationStep = { title: string; description: string; url?: string }

/** Os mesmos órgãos e links do passo a passo da contestação (`crates/rules/src/dispute.rs`). */
export const ESCALATION: Record<BillKind, { intro: string; steps: EscalationStep[] }> = {
  telecom: {
    intro: 'A operadora não respondeu a tempo? Leve o caso a quem fiscaliza. Tenha o protocolo em mãos.',
    steps: [
      {
        title: 'Anatel',
        description: 'Registre uma reclamação na Anatel e informe o protocolo do primeiro contato.',
        url: 'https://www.gov.br/anatel/pt-br/consumidor/reclamacoes',
      },
      {
        title: 'consumidor.gov.br',
        description: 'Se ainda não resolver, registre também no consumidor.gov.br. A empresa costuma responder em até 10 dias.',
        url: 'https://www.consumidor.gov.br',
      },
    ],
  },
  energy: {
    intro: 'A distribuidora não respondeu a tempo? Suba um degrau. Tenha o protocolo em mãos.',
    steps: [
      {
        title: 'Ouvidoria da distribuidora',
        description: 'Procure a ouvidoria da própria distribuidora e informe o protocolo do primeiro contato. Ela responde em até 10 dias úteis. O contato dela está na conta de luz e no site da empresa.',
      },
      {
        title: 'ANEEL',
        description: 'Se a ouvidoria não resolver, registre uma reclamação na ANEEL: telefone 167, app ANEEL Consumidor ou site.',
        url: 'https://www.gov.br/aneel/pt-br/consumidores/como-registrar-reclamacao',
      },
    ],
  },
}

/* ---------- Alerta de aumento ---------- */

export type Rise = {
  provider: string
  kind: BillKind
  fromId: string
  toId: string
  fromMonth: string
  toMonth: string
  /** Percentual arredondado, para exibir. */
  pct: number
}

/** "  Vivaz   Telecom " e "VIVAZ telecom" são a mesma empresa. */
const providerKey = (p: string) =>
  p
    .normalize('NFD')
    .replace(/\p{Diacritic}/gu, '')
    .toLowerCase()
    .replace(/\s+/g, ' ')
    .trim()

/**
 * Para cada empresa e tipo de conta, compara os dois meses mais recentes guardados. Avisa quando o
 * total subiu `RISE_THRESHOLD_PCT`% ou mais. É só aritmética sobre os totais: nada de regra jurídica.
 */
export function findRises(items: HistoryItem[]): Rise[] {
  const groups = new Map<string, Map<string, HistoryItem>>()
  for (const item of items) {
    const key = `${item.kind}|${providerKey(item.provider)}`
    const months = groups.get(key) ?? new Map<string, HistoryItem>()
    const same = months.get(item.month)
    // No mesmo mês, vale a conta vista por último (ex.: a pessoa corrigiu e abriu de novo).
    if (!same || Date.parse(item.seenAt) > Date.parse(same.seenAt)) months.set(item.month, item)
    groups.set(key, months)
  }
  const rises: Rise[] = []
  for (const months of groups.values()) {
    if (months.size < 2) continue
    const [latest, previous] = [...months.values()].sort((a, b) => b.month.localeCompare(a.month))
    if (previous.totalCents <= 0) continue
    const growth = latest.totalCents - previous.totalCents
    if (growth * 100 < RISE_THRESHOLD_PCT * previous.totalCents) continue
    rises.push({
      provider: latest.provider,
      kind: latest.kind,
      fromId: previous.id,
      toId: latest.id,
      fromMonth: previous.month,
      toMonth: latest.month,
      pct: Math.round((growth * 100) / previous.totalCents),
    })
  }
  return rises
}
