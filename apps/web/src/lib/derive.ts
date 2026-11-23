import type { EnergyData } from '../types/EnergyData'
import type { ExtractedBill } from '../types/ExtractedBill'
import type { ItemCategory } from '../types/ItemCategory'

const TOLERANCE_CENTS = 5

export function sumDiverges(bill: ExtractedBill): boolean {
  const sum = bill.items.reduce((acc, i) => acc + i.amount_cents, 0)
  return Math.abs(sum - bill.total_cents) > TOLERANCE_CENTS
}

export const monthName = (ym: string) =>
  new Intl.DateTimeFormat('pt-BR', { month: 'long', timeZone: 'UTC' }).format(new Date(`${ym}-01T00:00:00Z`))
export const monthShort = (ym: string) =>
  new Intl.DateTimeFormat('pt-BR', { month: 'short', timeZone: 'UTC' }).format(new Date(`${ym}-01T00:00:00Z`)).replace('.', '')

/** Resumo em texto: "Consumo de outubro: 324 kWh, 62% acima da média de 200 kWh". */
export function consumptionSummary(energy: EnergyData, referenceMonth: string): { text: string; avg: number } {
  const hist = energy.history
  const avg = hist.length ? Math.round(hist.reduce((a, p) => a + p.kwh, 0) / hist.length) : 0
  const base = `Consumo de ${monthName(referenceMonth)}: ${energy.consumption_kwh} kWh`
  if (!avg) return { text: base, avg }
  const pct = Math.round(((energy.consumption_kwh - avg) / avg) * 100)
  const cmp = pct === 0 ? 'igual à média' : `${Math.abs(pct)}% ${pct > 0 ? 'acima' : 'abaixo'} da média`
  return { text: `${base}, ${cmp}${pct === 0 ? '' : ' de'} ${pct === 0 ? `(${avg} kWh)` : `${avg} kWh`}`, avg }
}

export type CompositionKey = 'principal' | 'impostos' | 'bandeira' | 'iluminacao' | 'extras' | 'outros'

export type CompositionPart = {
  key: CompositionKey
  label: string
  cents: number
  /** Parte do total em décimos de ponto percentual (550 = 55,0%). A soma de todas as partes é sempre 1000. */
  permille: number
  /** Extras (serviço adicional) aparecem com a cor de alerta. */
  alert: boolean
}

const PART_ORDER: CompositionKey[] = ['principal', 'impostos', 'bandeira', 'iluminacao', 'extras', 'outros']

const PART_BY_CATEGORY: Record<ItemCategory, CompositionKey> = {
  energia: 'principal',
  plano: 'principal',
  imposto: 'impostos',
  bandeira: 'bandeira',
  iluminacao_publica: 'iluminacao',
  servico_adicional: 'extras',
  encargo_atraso: 'outros',
  // Tarifa de banco e de cartão (#54, #55): fica em "Outros", sem fatia própria.
  tarifa: 'outros',
  outros: 'outros',
}

const PART_LABEL: Record<Exclude<CompositionKey, 'principal'>, string> = {
  impostos: 'Impostos',
  bandeira: 'Bandeira',
  iluminacao: 'Iluminação pública',
  extras: 'Extras',
  outros: 'Outros',
}

/**
 * "Para onde vai seu dinheiro" (RF23): agrupa os itens por categoria.
 *
 * Garantias: a soma de `cents` é igual a `bill.total_cents` e a soma de `permille` é 1000.
 * - A diferença entre a soma dos itens e o total da fatura entra em "Outros".
 * - Partes zeradas não aparecem. Descontos (valor negativo) abatem da maior parte.
 * - O arredondamento usa o método do maior resto, então a porcentagem nunca passa de 100%.
 */
export function composition(bill: ExtractedBill): CompositionPart[] {
  const cents: Record<CompositionKey, number> = { principal: 0, impostos: 0, bandeira: 0, iluminacao: 0, extras: 0, outros: 0 }
  let itemsSum = 0
  for (const it of bill.items) {
    cents[PART_BY_CATEGORY[it.category]] += it.amount_cents
    itemsSum += it.amount_cents
  }
  cents.outros += bill.total_cents - itemsSum

  const parts = PART_ORDER.map((key) => ({ key, cents: cents[key] })).filter((p) => p.cents !== 0)
  const positive = parts.filter((p) => p.cents > 0)
  if (!positive.length) return []

  const discounts = parts.filter((p) => p.cents < 0).reduce((a, p) => a + p.cents, 0)
  const largest = positive.reduce((a, p) => (p.cents > a.cents ? p : a))
  largest.cents += discounts
  if (largest.cents <= 0) return []

  const total = positive.reduce((a, p) => a + p.cents, 0)
  const shares = positive.map((p) => ({ p, permille: Math.floor((p.cents * 1000) / total), rest: (p.cents * 1000) % total }))
  let left = 1000 - shares.reduce((a, s) => a + s.permille, 0)
  for (const s of [...shares].sort((a, b) => b.rest - a.rest)) {
    if (left <= 0) break
    s.permille += 1
    left -= 1
  }

  return shares.map(({ p, permille }) => ({
    key: p.key,
    label: p.key === 'principal' ? (bill.kind === 'energy' ? 'Energia' : 'Plano') : PART_LABEL[p.key],
    cents: p.cents,
    permille,
    alert: p.key === 'extras',
  }))
}
