import { formatBRL, formatMonth } from '../../lib/format'
import type { ExtractedBill } from '../../types/ExtractedBill'
import type { Finding } from '../../types/Finding'
import { Paper, PaperHeader, PaperTotal } from '../ui'
import { itemStatus, SEVERITY_META } from './severity'
import { SeverityBadge } from './SeverityBadge'

/** O achado mais grave que cita o item: é ele que dá a "parte do problema" da anotação. */
function worstFinding(itemId: string, findings: Finding[]): Finding | undefined {
  return findings
    .filter((f) => f.severity !== 'ok' && f.item_refs.includes(itemId))
    .sort((a, b) => SEVERITY_META[a.severity].rank - SEVERITY_META[b.severity].rank)[0]
}

/** "Sua conta explicada": a fatura em papel, com marca-texto nos itens indevidos e a anotação ao lado. */
export function BillItems({ bill, findings }: { bill: ExtractedBill; findings: Finding[] }) {
  return (
    <Paper>
      <PaperHeader>
        <span className="min-w-0 break-words">{bill.provider_name}</span>
        <span className="font-normal text-muted">{formatMonth(bill.reference_month)}</span>
      </PaperHeader>
      <ul>
        {bill.items.map((it) => {
          const status = itemStatus(it.id, findings)
          const problem = worstFinding(it.id, findings)
          const mark = status === 'indevido' ? 'marca-texto' : ''
          return (
            <li key={it.id} className="grid gap-2 border-b border-line px-4 py-3">
              <div className="flex items-start justify-between gap-3 font-semibold">
                <h3 className="min-w-0 font-sans text-body tracking-normal break-words">
                  <span className={mark}>{it.description}</span>
                </h3>
                <span className="whitespace-nowrap">
                  <span className={mark}>{formatBRL(it.amount_cents)}</span>
                </span>
              </div>
              <p className="border-l-2 border-accent pl-3 text-accent">
                {problem && (
                  <strong className={`font-bold ${problem.severity === 'indevido' ? 'text-danger' : 'text-warning'}`}>{problem.title.replace(/[.!?]+$/, '')}. </strong>
                )}
                {it.plain_explanation}
              </p>
              <div className="flex flex-wrap items-center justify-between gap-2">
                <SeverityBadge severity={status} />
                <p className="font-mono text-caption break-words text-muted">{it.original_text} · pág. {it.page}</p>
              </div>
            </li>
          )
        })}
      </ul>
      <PaperTotal>
        <span>Total</span>
        <span>{formatBRL(bill.total_cents)}</span>
      </PaperTotal>
    </Paper>
  )
}
