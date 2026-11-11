import type { Severity } from '../../types/Severity'
import { SEVERITY_META } from './severity'

function Icon({ severity }: { severity: Severity }) {
  const common = { width: 18, height: 18, viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor', strokeWidth: 2.2, strokeLinecap: 'round', strokeLinejoin: 'round', 'aria-hidden': true } as const
  if (severity === 'indevido')
    return (
      <svg {...common}>
        <circle cx="12" cy="12" r="9.5" />
        <path d="M9 9l6 6M15 9l-6 6" />
      </svg>
    )
  if (severity === 'suspeito')
    return (
      <svg {...common}>
        <path d="M12 3.5l9.5 16.5h-19z" />
        <path d="M12 10v4.5M12 17.4v.1" />
      </svg>
    )
  return (
    <svg {...common}>
      <circle cx="12" cy="12" r="9.5" />
      <path d="M7.8 12.4l2.9 2.9 5.5-6" />
    </svg>
  )
}

/** Severidade sempre com ícone + texto: nunca depende só de cor. */
export function SeverityBadge({ severity, compact = false }: { severity: Severity; compact?: boolean }) {
  const m = SEVERITY_META[severity]
  return (
    <span
      className={`inline-flex items-center gap-1.5 rounded-full border font-semibold whitespace-nowrap ${m.cls} ${compact ? 'px-2.5 py-0.5 text-xs' : 'px-3 py-1 text-sm'}`}
    >
      <Icon severity={severity} />
      {m.label}
    </span>
  )
}
