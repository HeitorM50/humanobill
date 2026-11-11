import { formatBRL } from '../../lib/format'
import type { BillItem } from '../../types/BillItem'
import type { Finding } from '../../types/Finding'
import { Card } from '../ui'
import { itemStatus } from './severity'
import { SeverityBadge } from './SeverityBadge'

export function BillItems({ items, findings }: { items: BillItem[]; findings: Finding[] }) {
  return (
    <ul className="space-y-3">
      {items.map((it) => (
        <li key={it.id}>
          <Card>
            <div className="flex items-start justify-between gap-3">
              <h3 className="font-semibold">{it.description}</h3>
              <span className="font-semibold whitespace-nowrap tabular-nums">{formatBRL(it.amount_cents)}</span>
            </div>
            <p className="mt-1 text-muted">{it.plain_explanation}</p>
            <div className="mt-3 flex flex-wrap items-center justify-between gap-2">
              <SeverityBadge severity={itemStatus(it.id, findings)} compact />
              <p className="font-mono text-xs break-words text-muted">{it.original_text} · pág. {it.page}</p>
            </div>
          </Card>
        </li>
      ))}
    </ul>
  )
}
