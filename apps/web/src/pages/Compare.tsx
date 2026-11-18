import { useEffect, useRef, useState, type FormEvent, type ReactNode } from 'react'
import { useMutation } from '@tanstack/react-query'
import { useSearchParams } from 'react-router'
import { Alert, Button, Card, Field, Icon, Page, Paper, PaperLine } from '../components/ui'
import { createComparison } from '../lib/api'
import { formatBRL, formatMonth } from '../lib/format'
import type { ComparedItem } from '../types/ComparedItem'
import type { Comparison } from '../types/Comparison'

const UUID = /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/i
const INVALID_ID = 'Não achei um link de resultado aqui. Cole o endereço da página do resultado ou o código dele.'

/** Aceita o link do resultado (`…/r/<código>`) ou o código puro. */
function extractId(text: string): string | null {
  return UUID.exec(text)?.[0].toLowerCase() ?? null
}

const pct = new Intl.NumberFormat('pt-BR', { minimumFractionDigits: 1, maximumFractionDigits: 1 })

/** "+R$ 9,90" / "−R$ 5,00". O sinal e a palavra ("a mais") dizem o sentido, nunca só a cor. */
function signed(cents: number): string {
  if (cents > 0) return `+${formatBRL(cents)}`
  if (cents < 0) return `−${formatBRL(-cents)}`
  return formatBRL(0)
}

const plural = (n: number) => `${n} ${n === 1 ? 'item' : 'itens'}`

/** Bloco de cor de ponta a ponta; o conteúdo respeita a largura máxima da página. */
const INNER = 'mx-auto max-w-page px-5 py-8 md:px-10 md:py-14'

/** Tag "novo este mês": ícone + texto, no mesmo formato dos selos (guia 5.5). */
function NewTag() {
  return (
    <span className="mr-2 inline-flex items-center gap-1.5 rounded-control border border-info bg-info-soft px-2.5 py-1 text-caption font-semibold whitespace-nowrap text-info">
      <Icon name="info" size={16} />
      Novo este mês
    </span>
  )
}

function ItemsSection({ title, items, note }: { title: string; items: ComparedItem[]; note: (i: ComparedItem) => ReactNode }) {
  if (items.length === 0) return null
  const headingId = `secao-${title.toLowerCase().replaceAll(/\W+/g, '-')}`
  return (
    <section aria-labelledby={headingId} className="space-y-3">
      <h2 id={headingId} className="text-h3">{title}</h2>
      <Paper>
        <ul>
          {items.map((i) => (
            <PaperLine key={`${i.description}-${i.before_cents}-${i.after_cents}`} label={i.description} value={signed(i.delta_cents)} highlight={i.new_charge}>
              {note(i)}
            </PaperLine>
          ))}
        </ul>
      </Paper>
    </section>
  )
}

const addedNote = (i: ComparedItem) => (
  <>
    <NewTag />
    {i.new_charge ? 'Cobrança nova: não aparecia na fatura anterior. Você pediu isso?' : 'Não aparecia na fatura anterior.'}
  </>
)

function changedNote(i: ComparedItem): ReactNode {
  const before = formatBRL(i.before_cents ?? 0)
  const after = formatBRL(i.after_cents ?? 0)
  const how = i.delta_pct === null ? '' : ` (${pct.format(Math.abs(i.delta_pct))}% a ${i.delta_cents > 0 ? 'mais' : 'menos'})`
  return `Era ${before} e agora é ${after}${how}.${i.new_charge ? ' Cobrança nova: antes não era cobrado. Você pediu isso?' : ''}`
}

const removedNote = (i: ComparedItem) => `Era ${formatBRL(i.before_cents ?? 0)} e saiu da conta.`

function TotalBlock({ c }: { c: Comparison }) {
  const heading = useRef<HTMLHeadingElement>(null)
  // O resultado aparece abaixo do formulário: leva o foco até ele para quem usa leitor de tela.
  useEffect(() => {
    heading.current?.focus({ preventScroll: true })
    heading.current?.scrollIntoView?.({ block: 'start' })
  }, [])

  const up = c.total_delta_cents > 0
  const title = up ? 'Sua conta subiu' : c.total_delta_cents < 0 ? 'Sua conta caiu' : 'Sua conta não mudou'
  const from = formatBRL(c.before_total_cents)
  const to = formatBRL(c.after_total_cents)
  const support =
    c.total_delta_cents === 0
      ? `O total continua ${to}.`
      : `De ${from} para ${to}${c.total_delta_pct === null ? '' : `, ${pct.format(Math.abs(c.total_delta_pct))}% a ${up ? 'mais' : 'menos'}`}.`

  return (
    <section aria-label="Variação do total" className="bg-accent text-on-accent">
      <div className={INNER}>
        <p className="text-caption">{c.provider_name} · de {formatMonth(c.before_month)} para {formatMonth(c.after_month)}</p>
        <h2 ref={heading} tabIndex={-1} className="mt-1 text-h4 focus:outline-none">{title}</h2>
        {/* O marca-texto só entra quando a conta subiu: é o que merece atenção. */}
        <p className="mt-2 text-hero tabular-nums">
          <span className={up ? 'marca-texto' : undefined}>{formatBRL(Math.abs(c.total_delta_cents))}</span>
        </p>
        <p className="mt-3 max-w-md text-h4 leading-normal">{support}</p>
      </div>
    </section>
  )
}

function Details({ c }: { c: Comparison }) {
  const nothingChanged = c.added.length + c.changed.length + c.removed.length === 0
  return (
    <Page width="form" className="space-y-8">
      {nothingChanged && (
        <Alert tone="info">
          <p>Os itens são os mesmos nos dois meses.</p>
        </Alert>
      )}
      <ItemsSection title="Itens novos" items={c.added} note={addedNote} />
      <ItemsSection title="Mudou de valor" items={c.changed} note={changedNote} />
      <ItemsSection title="Saiu da conta" items={c.removed} note={removedNote} />
      {c.unchanged.length > 0 && (
        <details className="rounded-card border border-line bg-surface">
          <summary className="min-h-12 cursor-pointer px-4 py-3 font-semibold">Sem mudança ({plural(c.unchanged.length)})</summary>
          <ul className="border-t border-line">
            {c.unchanged.map((i) => (
              <PaperLine key={i.description} label={i.description} value={formatBRL(i.after_cents ?? 0)} />
            ))}
          </ul>
        </details>
      )}
      <p className="max-w-text text-caption text-muted">
        A conta é feita só com os valores das duas faturas, sem IA. Nada disso é guardado.
      </p>
    </Page>
  )
}

export default function Compare() {
  const [params] = useSearchParams()
  const [first, setFirst] = useState(() => params.get('a') ?? '')
  const [second, setSecond] = useState(() => params.get('b') ?? '')
  const [errors, setErrors] = useState<{ first?: string; second?: string }>({})
  const firstRef = useRef<HTMLInputElement>(null)
  const secondRef = useRef<HTMLInputElement>(null)
  const comparison = useMutation({ mutationFn: createComparison })

  const submit = (e: FormEvent) => {
    e.preventDefault()
    const a = extractId(first)
    const b = extractId(second)
    setErrors({ first: a ? undefined : INVALID_ID, second: b ? undefined : INVALID_ID })
    if (!a || !b) {
      ;(a ? secondRef : firstRef).current?.focus()
      return
    }
    comparison.mutate([a, b])
  }

  return (
    <>
      <Page width="form" className="space-y-6">
        <header className="space-y-3">
          <h1 className="text-h1">Comparar duas faturas</h1>
          <p className="max-w-text">
            Veja o que subiu, o que é novo e o que saiu da sua conta. Use duas faturas da mesma empresa, de meses diferentes.
          </p>
        </header>
        <Card>
          <form onSubmit={submit} noValidate className="grid gap-5">
            <Field label="Primeira fatura" hint="Cole o link do resultado dessa fatura, ou só o código dele." error={errors.first}>
              {(c) => <input {...c} ref={firstRef} type="text" autoComplete="off" spellCheck={false} value={first} onChange={(e) => setFirst(e.target.value)} />}
            </Field>
            <Field label="Segunda fatura" hint="Pode ser a mais nova ou a mais antiga. A ordem não importa." error={errors.second}>
              {(c) => <input {...c} ref={secondRef} type="text" autoComplete="off" spellCheck={false} value={second} onChange={(e) => setSecond(e.target.value)} />}
            </Field>
            <Button type="submit" loading={comparison.isPending} loadingLabel="Comparando…" className="w-full sm:w-fit">
              Comparar
            </Button>
          </form>
        </Card>
        <p className="max-w-text text-caption text-muted">
          Onde acho o link? Abra o resultado de cada fatura e use “Copiar link”.
        </p>
        {comparison.isError && (
          <Alert tone="danger" role="alert">
            <p>{comparison.error.message}</p>
          </Alert>
        )}
      </Page>
      {comparison.data && !comparison.isPending && (
        <>
          <TotalBlock c={comparison.data} />
          <Details c={comparison.data} />
        </>
      )}
    </>
  )
}
