import type { EnergyData } from '../types/EnergyData'
import type { ExtractedBill } from '../types/ExtractedBill'

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

