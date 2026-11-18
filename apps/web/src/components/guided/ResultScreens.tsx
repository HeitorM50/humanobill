import allClearUrl from '../../assets/tudo-certo.svg'
import { formatBRL, formatMonth } from '../../lib/format'
import type { Analysis } from '../../types/Analysis'
import type { BillItem } from '../../types/BillItem'
import type { Finding } from '../../types/Finding'
import { CorrectionDrawer } from '../result/CorrectionDrawer'
import { ListenButton } from '../result/ListenButton'
import { SeverityBadge } from '../result/SeverityBadge'
import { valueSummary } from '../result/valueSummary'
import { Alert, Card, Icon } from '../ui'

/** Etapa 3, primeira tela: o total, quanto parece indevido e o "Ouvir". */
export function SummaryScreen({ analysis }: { analysis: Analysis }) {
  const { bill } = analysis
  const value = valueSummary(analysis)
  const pending = analysis.questions.filter((q) => !(q.id in analysis.answers)).length
  const charges = value ? `${value.count} ${value.count === 1 ? 'cobrança' : 'cobranças'}` : ''

  return (
    <>
      <Card className="space-y-1">
        <p className="text-muted">
          {bill.provider_name} · {formatMonth(bill.reference_month)}
        </p>
        <p className="text-h4 font-semibold">Total da fatura</p>
        <p className="font-display text-h1 font-bold tracking-display">{formatBRL(bill.total_cents)}</p>
      </Card>

      {value ? (
        <section aria-label={value.label} className="space-y-2 rounded-card bg-accent p-6 text-on-accent">
          <p className="text-h4 font-semibold">{value.label}</p>
          <p className="flex flex-wrap items-baseline gap-x-3 font-display text-h1 font-bold tracking-display">
            {value.lead && <span className="text-h3">{value.lead}</span>}
            <span className="marca-texto">{formatBRL(value.cents)}</span>
          </p>
          {value.tail && <p className="text-h4 font-semibold">{value.tail}</p>}
          <p>em {charges} desta fatura</p>
        </section>
      ) : (
        <Alert tone="ok" title="Tudo certo com esta conta">
          <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:gap-4">
            <img src={allClearUrl} alt="" aria-hidden="true" width={84} height={84} className="size-21 shrink-0" />
            <div className="space-y-2">
              <p>Não achei nada fora do lugar nesta fatura.</p>
              {pending > 0 && (
                <p className="font-semibold">
                  Ainda tem {pending === 1 ? 'uma pergunta' : `${pending} perguntas`} sem resposta. Responder pode mudar este resultado.
                </p>
              )}
            </div>
          </div>
        </Alert>
      )}

      <div className="flex flex-wrap items-center gap-3">
        <ListenButton analysis={analysis} />
        <CorrectionDrawer analysis={analysis} />
      </div>
      <p className="text-caption text-muted">Conteúdo informativo, não é orientação jurídica.</p>
    </>
  )
}

/** Etapa 3, um problema por tela, em linguagem simples. */
export function FindingScreen({ finding, items }: { finding: Finding; items: BillItem[] }) {
  return (
    <Card className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <SeverityBadge severity={finding.severity} />
        {finding.amount_cents > 0 && <p className="font-display text-h2 font-bold tracking-display">{formatBRL(finding.amount_cents)}</p>}
      </div>
      <h2 className="font-sans text-h3 tracking-normal">{finding.title}</h2>
      <p className="text-h4 font-normal">{finding.explanation}</p>
      {items.length > 0 && (
        <p className="text-muted">
          {items.length > 1 ? 'Itens: ' : 'Item: '}
          <span className="font-semibold text-ink">{items.map((i) => i.description).join(', ')}</span>
        </p>
      )}
      {finding.legal_basis.length > 0 && (
        <div>
          <p className="font-semibold text-muted">O que diz a lei</p>
          <ul className="mt-1 flex flex-wrap gap-x-4">
            {finding.legal_basis.map((l) => (
              <li key={l.url}>
                <a
                  href={l.url}
                  target="_blank"
                  rel="noopener noreferrer"
                  aria-label={`${l.label} (abre em nova aba)`}
                  className="inline-flex min-h-12 items-center gap-1.5 font-semibold text-accent underline underline-offset-4 hover:text-accent-strong"
                >
                  {l.label}
                  <Icon name="external" size={16} />
                </a>
              </li>
            ))}
          </ul>
        </div>
      )}
    </Card>
  )
}
