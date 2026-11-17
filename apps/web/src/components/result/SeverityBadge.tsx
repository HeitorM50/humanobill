import type { Severity } from '../../types/Severity'
import { Icon, type IconName } from '../ui'
import { SEVERITY_META } from './severity'

const ICON: Record<Severity, IconName> = { indevido: 'danger', suspeito: 'warning', ok: 'ok' }

/** Selo (guia 5.5): severidade sempre com ícone + texto, nunca só cor. */
export function SeverityBadge({ severity }: { severity: Severity }) {
  const m = SEVERITY_META[severity]
  return (
    <span className={`inline-flex items-center gap-1.5 rounded-control border px-2.5 py-1 text-caption font-semibold whitespace-nowrap ${m.cls}`}>
      <Icon name={ICON[severity]} size={16} />
      {m.label}
    </span>
  )
}
