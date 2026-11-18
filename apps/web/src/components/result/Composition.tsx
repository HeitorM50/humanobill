import { composition, type CompositionKey } from '../../lib/derive'
import { formatBRL } from '../../lib/format'
import type { ExtractedBill } from '../../types/ExtractedBill'
import { Card, Icon } from '../ui'

/** Cor de cada parte. Só tokens do guia. Extras usa a cor de alerta. */
const COLOR: Record<CompositionKey, string> = {
  principal: 'bg-accent',
  impostos: 'bg-muted',
  bandeira: 'bg-field-line',
  iluminacao: 'bg-info-line',
  extras: 'bg-warning',
  outros: 'bg-line',
}

/** 550 -> "55%", 62 -> "6,2%". */
const formatPermille = (permille: number) => `${(permille / 10).toLocaleString('pt-BR', { maximumFractionDigits: 1 })}%`

/**
 * "Para onde vai seu dinheiro" (RF23): barra empilhada com a lista de texto ao lado.
 * A barra é só ilustração (aria-hidden): a lista traz cor, nome, reais e porcentagem.
 */
export function Composition({ bill }: { bill: ExtractedBill }) {
  const parts = composition(bill)
  if (!parts.length) return null
  const total = parts.reduce((a, p) => a + p.cents, 0)

  return (
    <section aria-labelledby="composicao-t" className="space-y-4">
      <h2 id="composicao-t" className="text-h2">Para onde vai seu dinheiro</h2>
      <Card className="space-y-5">
        <div data-grafico aria-hidden="true" className="flex h-8 gap-0.5 overflow-hidden rounded-control">
          {parts.map((p) => (
            <span key={p.key} className={`min-w-1 basis-0 ${COLOR[p.key]}`} style={{ flexGrow: p.permille }} />
          ))}
        </div>
        <ul>
          {parts.map((p) => (
            <li
              key={p.key}
              data-alerta={p.alert ? 'true' : undefined}
              className="flex items-center gap-3 border-b border-line py-3 first:pt-0"
            >
              <span data-cor aria-hidden="true" className={`size-4 shrink-0 rounded-sm ring-1 ring-ink/25 ring-inset ${COLOR[p.key]}`} />
              <span className={`flex min-w-0 flex-1 items-center gap-1.5 break-words ${p.alert ? 'font-bold text-warning' : 'font-semibold'}`}>
                {p.label}
                {p.alert && <Icon name="warning" size={16} />}
              </span>
              <span className="whitespace-nowrap font-semibold">{formatBRL(p.cents)}</span>
              <span className="w-16 text-right whitespace-nowrap text-muted">{formatPermille(p.permille)}</span>
            </li>
          ))}
        </ul>
        {parts.some((p) => p.alert) && (
          <p className="text-caption text-muted">Extras são serviços cobrados além do que você usa. Confira se você contratou todos.</p>
        )}
        <p className="flex items-center gap-3 font-bold">
          <span className="flex-1">Total</span>
          <span className="whitespace-nowrap">{formatBRL(total)}</span>
          <span className="w-16 text-right whitespace-nowrap">100%</span>
        </p>
      </Card>
    </section>
  )
}
