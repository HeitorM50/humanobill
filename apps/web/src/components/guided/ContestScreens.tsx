import { formatBRL } from '../../lib/format'
import type { Analysis } from '../../types/Analysis'
import type { Finding } from '../../types/Finding'
import { CallScript, CopyText, Roadmap } from '../result/DisputeSection'
import { Alert, Button, Card, Spinner } from '../ui'
import { contestable, type Dispute } from './useDispute'

/** Contestar, primeira tela: marcar o que entra na contestação. */
export function ChooseScreen({
  analysis,
  isChecked,
  onChange,
}: {
  analysis: Analysis
  isChecked: (f: Finding) => boolean
  onChange: (id: string, checked: boolean) => void
}) {
  const list = contestable(analysis)
  const marked = list.filter(isChecked)
  const total = marked.reduce((t, f) => t + f.amount_cents, 0)
  return (
    <>
      <fieldset className="space-y-3">
        <legend className="mb-3 max-w-text text-h4">Marque o que você quer que a empresa corrija.</legend>
        {list.map((f) => (
          <label
            key={f.id}
            className="flex min-h-14 cursor-pointer items-start gap-3 rounded-card border border-line bg-surface p-4 has-checked:border-accent has-checked:bg-accent-soft"
          >
            <input type="checkbox" checked={isChecked(f)} onChange={(e) => onChange(f.id, e.target.checked)} className="mt-1 size-6 shrink-0 accent-accent" />
            <span className="min-w-0 flex-1">
              <span className="block font-semibold break-words">{f.title}</span>
              {f.amount_cents > 0 && <span className="block text-muted">{formatBRL(f.amount_cents)}</span>}
            </span>
          </label>
        ))}
      </fieldset>
      <p aria-live="polite" className="text-h4 font-bold">
        {marked.length === 0
          ? 'Marque pelo menos um problema para continuar.'
          : `${marked.length} ${marked.length === 1 ? 'problema marcado' : 'problemas marcados'} · ${formatBRL(total)}`}
      </p>
    </>
  )
}

/** Enquanto o texto não chega (ou se der erro), as telas seguintes mostram isto no lugar do conteúdo. */
function Waiting({ dispute }: { dispute: Dispute }) {
  if (dispute.error) {
    return (
      <Alert tone="danger" role="alert">
        <p>{dispute.error.message}</p>
        <Button variant="secondary" className="min-h-14!" onClick={dispute.request}>Tentar de novo</Button>
      </Alert>
    )
  }
  return (
    <p role="status" className="flex items-center gap-2 font-semibold">
      <Spinner />
      Preparando o seu texto…
    </p>
  )
}

/** Contestar, segunda tela: o texto pronto, para copiar e colar. */
export function TextScreen({ dispute }: { dispute: Dispute }) {
  const d = dispute.dispute
  if (!d) return <Waiting dispute={dispute} />
  return (
    <>
      <p className="max-w-text text-muted">
        Total contestado: <span className="font-semibold text-ink">{formatBRL(d.total_cents)}</span>. Toque em Copiar e cole no chat, no e-mail ou no formulário da empresa.
      </p>
      <CopyText text={d.text} />
      <Alert tone="info" className="max-w-text">
        <p>{d.disclaimer}</p>
      </Alert>
    </>
  )
}

/** Contestar, terceira tela: prazos e onde reclamar se a empresa não resolver. */
export function WhereScreen({ dispute }: { dispute: Dispute }) {
  const d = dispute.dispute
  if (!d) return <Waiting dispute={dispute} />
  return <Roadmap dispute={d} />
}

/** Contestar, última tela: o roteiro para quem prefere ligar (RF17). */
export function CallScreen({ dispute }: { dispute: Dispute }) {
  const d = dispute.dispute
  if (!d) return <Waiting dispute={dispute} />
  return (
    <Card className="space-y-4">
      <CallScript steps={d.call_script} stale={false} />
    </Card>
  )
}
