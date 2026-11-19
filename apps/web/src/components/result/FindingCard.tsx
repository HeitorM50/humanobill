import { useId, useState } from 'react'
import { motion } from 'motion/react'
import { formatBRL } from '../../lib/format'
import type { BillItem } from '../../types/BillItem'
import type { Finding } from '../../types/Finding'
import { Alert, Button, Icon } from '../ui'
import { SeverityBadge } from './SeverityBadge'

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
      className="rounded-card border border-line bg-surface p-5"
    >
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
          <p className="font-display text-h3 font-bold tracking-display">{formatBRL(f.amount_cents)}</p>
        )}
      </div>

      <h3 id={titleId} className="mt-3 font-sans text-h4 tracking-normal">{f.title}</h3>
      <p className="mt-1 text-muted">{f.explanation}</p>

      {items.length > 0 && (
        <p className="mt-3 text-caption text-muted">
          {items.length > 1 ? 'Itens: ' : 'Item: '}
          <span className="font-semibold text-ink">{items.map((i) => i.description).join(', ')}</span>
        </p>
      )}

      {f.original_text && (
        <figure className="mt-4 rounded-control border border-line bg-bg p-3">
          <figcaption className="text-caption font-semibold text-muted">
            Trecho original{f.page != null && <> · página {f.page}</>}
          </figcaption>
          <blockquote className="mt-1 font-mono text-caption break-words text-ink">{f.original_text}</blockquote>
        </figure>
      )}

      {f.legal_basis.length > 0 && (
        <div className="mt-4">
          <p className="text-caption font-semibold text-muted">Base legal</p>
          <ul className="mt-1 flex flex-wrap gap-x-4 gap-y-1">
            {f.legal_basis.map((l) => (
              <li key={l.url}>
                <a
                  href={l.url}
                  target="_blank"
                  rel="noopener noreferrer"
                  aria-label={`${l.label} (abre em nova aba)`}
                  className="inline-flex min-h-11.5 items-center gap-1.5 font-semibold text-accent underline underline-offset-4 hover:text-accent-strong"
                >
                  {l.label}
                  <Icon name="external" size={16} />
                </a>
              </li>
            ))}
          </ul>
        </div>
      )}

      {selection && f.severity !== 'ok' && (
        <label className="mt-4 flex min-h-11.5 cursor-pointer items-center gap-3 rounded-control border border-line bg-bg px-3 py-2 font-semibold has-checked:border-accent has-checked:bg-accent-soft">
          <input
            type="checkbox"
            checked={selection.checked}
            onChange={(e) => selection.onChange(e.target.checked)}
            className="size-5.5 shrink-0 accent-accent"
          />
          Incluir na contestação
        </label>
      )}

      <div className="mt-4 border-t border-line pt-2">
        <Button variant="ghost" aria-expanded={open} aria-controls={panelId} onClick={() => setOpen((v) => !v)}>
          <Icon name="chevron" size={16} className={`transition-transform duration-200 ease-out-soft ${open ? 'rotate-90' : ''}`} />
          Por que isso foi marcado?
        </Button>
        {open && (
          <Alert tone="info" id={panelId} className="mt-2">
            <p>A IA só leu sua fatura. Quem marcou foi a regra {f.rule_id}, que compara com a lei.</p>
          </Alert>
        )}
      </div>
    </motion.article>
  )
}
