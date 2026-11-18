import { formatBRL } from '../../lib/format'
import type { Analysis } from '../../types/Analysis'

const sum = (a: Analysis, sev: 'indevido' | 'suspeito') =>
  a.findings.filter((f) => f.severity === sev).reduce((t, f) => t + f.amount_cents, 0)

/** O número grande do resultado e as frases que o acompanham. Vem dos achados e de `recoverable_cents`, sem conta nova. */
export type ValueSummary = {
  kind: 'indevido' | 'suspeito'
  label: string
  /** "Até", quando já se sabe se pagou. */
  lead: string | null
  cents: number
  tail?: string
  /** A mesma frase, para leitor de tela. */
  spoken: string
  /** Quantas cobranças entram no valor. */
  count: number
}

/** `null` quando não há nada indevido nem suspeito. */
export function valueSummary(analysis: Analysis): ValueSummary | null {
  const undueFindings = analysis.findings.filter((f) => f.severity === 'indevido')
  if (undueFindings.length === 0) {
    const suspectFindings = analysis.findings.filter((f) => f.severity === 'suspeito')
    if (suspectFindings.length === 0) return null
    const suspect = sum(analysis, 'suspeito')
    return {
      kind: 'suspeito',
      label: 'Para conferir',
      lead: null,
      cents: suspect,
      spoken: `${formatBRL(suspect)} para conferir`,
      count: suspectFindings.length,
    }
  }

  const paid = analysis.paid
  const cents = paid === null ? sum(analysis, 'indevido') : analysis.recoverable_cents
  const lead = paid === null ? null : 'Até'
  const tail =
    paid === null
      ? 'parecem indevidos'
      : paid
        ? 'recuperáveis (devolução em dobro, CDC art. 42)'
        : 'podem ser retirados da fatura'
  return {
    kind: 'indevido',
    label: paid === null ? 'Parece indevido' : 'Valor recuperável',
    lead,
    cents,
    tail,
    spoken: `${lead ? lead + ' ' : ''}${formatBRL(cents)} ${tail}`,
    count: undueFindings.length,
  }
}
