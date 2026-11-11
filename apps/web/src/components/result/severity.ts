import type { Finding } from '../../types/Finding'
import type { Severity } from '../../types/Severity'

export const SEVERITY_META: Record<Severity, { label: string; cls: string; rank: number }> = {
  indevido: { label: 'Indevido', cls: 'bg-danger-soft text-danger border-danger-line', rank: 0 },
  suspeito: { label: 'Suspeito — confira', cls: 'bg-warning-soft text-warning border-warning-line', rank: 1 },
  ok: { label: 'Tudo certo', cls: 'bg-ok-soft text-ok border-ok-line', rank: 2 },
}

/** Pior severidade entre os achados que citam o item; sem achado = ok. */
export function itemStatus(itemId: string, findings: Finding[]): Severity {
  let worst: Severity = 'ok'
  for (const f of findings)
    if (f.item_refs.includes(itemId) && SEVERITY_META[f.severity].rank < SEVERITY_META[worst].rank) worst = f.severity
  return worst
}
