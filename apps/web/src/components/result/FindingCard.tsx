import { useId, useState } from 'react'
import { motion } from 'motion/react'
import { formatBRL } from '../../lib/format'
import type { BillItem } from '../../types/BillItem'
import type { Finding } from '../../types/Finding'
import { SeverityBadge } from './SeverityBadge'

const EDGE = { indevido: 'bg-danger', suspeito: 'bg-warning-line', ok: 'bg-ok-line' } as const

function ExternalIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M14 4h6v6M20 4l-9 9M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5" />
    </svg>
  )
}

type Selection = { checked: boolean; onChange: (checked: boolean) => void }

export function FindingCard({ finding, items, selection }: { finding: Finding; items: BillItem[]; selection?: Selection }) {
  const [open, setOpen] = useState(false)
  const panelId = useId()
  const titleId = useId()
  const f = finding

  return (
    <motion.article
      layout="position"
      aria-labelledby={titleId}
      className="relative overflow-hidden rounded-card border border-line bg-surface p-5 pl-6 shadow-card"
    >
      {/* Faixa lateral: reforço visual, a severidade em si está no selo (ícone + texto). */}
      <motion.span
        aria-hidden="true"
        key={f.severity}
        initial={{ scaleY: 0 }}
        animate={{ scaleY: 1 }}
        transition={{ duration: 0.35 }}
        style={{ originY: 0 }}
        className={`absolute inset-y-0 left-0 w-1.5 ${EDGE[f.severity]}`}
      />
      <div className="flex flex-wrap items-center justify-between gap-2">
        <motion.div
          key={f.severity}
          initial={{ opacity: 0, scale: 0.85 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={{ type: 'spring', stiffness: 380, damping: 22 }}
        >
          <SeverityBadge severity={f.severity} />
        </motion.div>
        {f.amount_cents > 0 && (
          <p className="text-xl font-bold tabular-nums">{formatBRL(f.amount_cents)}</p>
        )}
      </div>

      <h3 id={titleId} className="mt-3 text-lg leading-snug font-semibold">{f.title}</h3>
      <p className="mt-1 text-muted">{f.explanation}</p>

      {items.length > 0 && (
        <p className="mt-3 text-sm text-muted">
          {items.length > 1 ? 'Itens: ' : 'Item: '}
          <span className="font-semibold text-ink">{items.map((i) => i.description).join(', ')}</span>
        </p>
      )}

      {f.original_text && (
        <figure className="mt-4 rounded-control border border-line bg-bg p-3">
          <figcaption className="text-xs font-semibold tracking-wide text-muted uppercase">
            Trecho original{f.page != null && <> · página {f.page}</>}
          </figcaption>
          <blockquote className="mt-1 font-mono text-sm break-words text-ink">{f.original_text}</blockquote>
        </figure>
      )}

      {f.legal_basis.length > 0 && (
        <div className="mt-4">
          <p className="text-xs font-semibold tracking-wide text-muted uppercase">Base legal</p>
          <ul className="mt-1 flex flex-wrap gap-x-4 gap-y-1">
            {f.legal_basis.map((l) => (
              <li key={l.url}>
                <a
                  href={l.url}
                  target="_blank"
                  rel="noopener noreferrer"
                  aria-label={`${l.label} (abre em nova aba)`}
                  className="inline-flex min-h-11 items-center gap-1.5 font-semibold text-accent underline underline-offset-2 hover:text-accent-strong"
                >
                  {l.label}
                  <ExternalIcon />
                </a>
              </li>
            ))}
          </ul>
        </div>
      )}

      {selection && f.severity !== 'ok' && (
        <label className="mt-4 flex min-h-11 cursor-pointer items-center gap-3 rounded-control border border-line bg-bg px-3 py-2 font-semibold has-checked:border-accent has-checked:bg-accent-soft">
          <input
            type="checkbox"
            checked={selection.checked}
            onChange={(e) => selection.onChange(e.target.checked)}
            className="size-6 shrink-0 accent-accent"
          />
          Incluir na contestação
        </label>
      )}

      <div className="mt-2 border-t border-line pt-1">
        <button
          type="button"
          aria-expanded={open}
          aria-controls={panelId}
          onClick={() => setOpen((v) => !v)}
          className="inline-flex min-h-11 items-center gap-2 font-semibold text-accent hover:text-accent-strong"
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" className={`transition-transform ${open ? 'rotate-90' : ''}`}>
            <path d="M9 6l6 6-6 6" />
          </svg>
          Por que isso foi marcado?
        </button>
        {open && (
          <p id={panelId} className="mb-2 rounded-control bg-accent-soft p-3 text-sm text-ink">
            A IA só leu sua fatura. Quem marcou foi a regra {f.rule_id}, que compara com a lei.
          </p>
        )}
      </div>
    </motion.article>
  )
}
