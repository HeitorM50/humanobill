import { useRef, useState } from 'react'
import { Link } from 'react-router'
import { Alert, Button, ButtonLink, Card, Icon, Page } from '../components/ui'
import { formatBRL, formatMonth } from '../lib/format'
import {
  ENERGY_VISIT_DAYS,
  ESCALATION,
  clearHistory,
  disputeDeadline,
  findRises,
  historyAvailable,
  isExpired,
  removeItem,
  useHistory,
  type HistoryItem,
} from '../lib/history'

const KIND_LABEL = { telecom: 'Celular/Internet', energy: 'Luz' } as const

const dateFormat = new Intl.DateTimeFormat('pt-BR', { day: '2-digit', month: '2-digit', year: 'numeric' })
const formatDate = (d: Date) => dateFormat.format(d)
/** "2026-12-01" (data do calendário, sem fuso) -> "01/12/2026". */
const formatDay = (iso: string) => {
  const [y, m, d] = iso.split('-').map(Number)
  return formatDate(new Date(y, m - 1, d))
}

const LINK = 'inline-flex min-h-11.5 items-center gap-1.5 font-semibold text-accent underline underline-offset-4 hover:text-accent-strong'

function ExternalLink({ href, title }: { href: string; title: string }) {
  return (
    <a href={href} target="_blank" rel="noopener noreferrer" aria-label={`${title} (abre em nova aba)`} className={LINK}>
      Abrir site
      <Icon name="external" size={16} />
    </a>
  )
}

/** Quando o prazo da empresa passa, mostra a escalada certa para o tipo de conta. */
function Escalation({ item }: { item: HistoryItem }) {
  const { intro, steps } = ESCALATION[item.kind]
  return (
    <Alert tone="warning" title="Prazo vencido: próximo passo">
      <p>{intro}</p>
      <ol className="space-y-3">
        {steps.map((s, i) => (
          <li key={s.title} className="flex gap-3">
            <span aria-hidden="true" className="flex size-8 shrink-0 items-center justify-center rounded-full bg-accent font-bold text-on-accent">
              {i + 1}
            </span>
            <div className="min-w-0">
              <p className="font-semibold text-ink">{s.title}</p>
              <p className="text-ink">{s.description}</p>
              {s.url && <ExternalLink href={s.url} title={s.title} />}
            </div>
          </li>
        ))}
      </ol>
    </Alert>
  )
}

function DeadlineBlock({ item, now }: { item: HistoryItem; now: Date }) {
  const deadline = disputeDeadline(item, now)
  if (!item.dispute || !deadline) return null
  const until = formatDay(deadline.deadline)
  const limit = deadline.business ? `${deadline.limitDays} dias úteis` : `${deadline.limitDays} dias`
  const who = item.kind === 'energy' ? 'da distribuidora' : 'da empresa'
  const unit = (n: number) => (deadline.business ? (n === 1 ? 'dia útil' : 'dias úteis') : n === 1 ? 'dia' : 'dias')
  // Na energia, a conta só pula sábado e domingo: feriado não é descontado. Com visita técnica o prazo é maior.
  const note = deadline.business && (
    <p className="text-caption">
      Conta só dias de semana, sem descontar feriados. Com visita técnica, a distribuidora tem {ENERGY_VISIT_DAYS} dias úteis.
    </p>
  )
  return (
    <div className="space-y-3">
      <p className="text-caption text-muted">
        Contestação feita em {formatDate(new Date(item.dispute.at))} · protocolo <span className="font-semibold text-ink tabular-nums">{item.dispute.protocol}</span>
      </p>
      {deadline.status === 'aberto' && (
        <Alert tone="info">
          <p>
            {deadline.daysLeft === 1 ? 'Falta 1' : `Faltam ${deadline.daysLeft}`} {unit(deadline.daysLeft)} do prazo de {limit} {who} (até {until}).
          </p>
          {note}
        </Alert>
      )}
      {deadline.status === 'hoje' && (
        <Alert tone="warning">
          <p>Hoje é o último dia do prazo de {limit} {who} ({until}). Se ela não responder, amanhã você já pode escalar.</p>
          {note}
        </Alert>
      )}
      {deadline.status === 'vencido' && <Escalation item={item} />}
    </div>
  )
}

function BillCard({ item, now, onRemove }: { item: HistoryItem; now: Date; onRemove: (item: HistoryItem) => void }) {
  const expired = isExpired(item, now)
  const month = formatMonth(item.month)
  return (
    <Card className="space-y-4">
      <div className="flex flex-wrap items-start justify-between gap-x-4 gap-y-1">
        <div className="min-w-0">
          <h3 className="text-h3">{item.provider}</h3>
          <p className="text-muted">{KIND_LABEL[item.kind]} · {month}</p>
        </div>
        <p className="text-h3 tabular-nums">{formatBRL(item.totalCents)}</p>
      </div>
      <p className="text-caption text-muted">
        Visto em {formatDate(new Date(item.seenAt))}
        {item.recoverableCents > 0 && (
          <>
            {' '}· Dá para recuperar <span className="font-semibold text-ink tabular-nums">{formatBRL(item.recoverableCents)}</span>
          </>
        )}
      </p>
      <DeadlineBlock item={item} now={now} />
      {expired && (
        <p className="flex items-center gap-2 font-semibold text-info">
          <Icon name="info" size={16} />
          Resultado expirado. Para ver de novo, analise a fatura outra vez.
        </p>
      )}
      <div className="flex flex-wrap items-center gap-x-4 gap-y-1">
        {expired ? (
          <Link to="/analisar" className={LINK}>Analisar de novo</Link>
        ) : (
          <Link to={`/r/${encodeURIComponent(item.id)}`} className={LINK} aria-label={`Abrir o resultado de ${item.provider}, ${month}`}>
            Abrir resultado
          </Link>
        )}
        <Button variant="ghost" onClick={() => onRemove(item)} aria-label={`Remover ${item.provider}, ${month}, da lista`}>
          Remover
        </Button>
      </div>
    </Card>
  )
}

export default function MyBills() {
  const items = useHistory()
  const [available] = useState(historyAvailable)
  const [confirming, setConfirming] = useState(false)
  const [notice, setNotice] = useState('')
  const heading = useRef<HTMLHeadingElement>(null)
  // Fixo durante a visita: a contagem de dias não precisa mudar com a página aberta.
  const [now] = useState(() => new Date())

  const rises = findRises(items)
  const byId = new Map(items.map((i) => [i.id, i]))

  const afterChange = (message: string) => {
    setNotice(message)
    // O botão que estava aqui some: leva o foco ao título para não perder o lugar.
    heading.current?.focus()
  }

  return (
    <Page width="form" className="space-y-8">
      <header className="space-y-3">
        <h1 ref={heading} tabIndex={-1} className="text-h1 outline-none">Minhas contas neste aparelho</h1>
        <p className="flex max-w-text items-start gap-2 text-muted">
          <Icon name="lock" size={16} className="mt-1.5" />
          Fica só neste aparelho. Apagar o histórico do navegador apaga esta lista.
        </p>
      </header>

      {!available && (
        <Alert tone="warning" title="Este navegador não deixa guardar a lista">
          <p>Pode ser a janela anônima ou o bloqueio de dados do site. Suas análises funcionam do mesmo jeito, só não ficam salvas aqui.</p>
        </Alert>
      )}

      <p role="status" className="sr-only">{notice}</p>

      {available && items.length === 0 && (
        <Card className="space-y-4">
          <p className="text-h4 font-bold">Nenhuma conta por aqui ainda.</p>
          <p className="max-w-text text-muted">Quando você abrir o resultado de uma fatura, ela aparece nesta lista, com o prazo da sua contestação.</p>
          <ButtonLink to="/analisar">Analisar uma conta</ButtonLink>
        </Card>
      )}

      {rises.length > 0 && (
        <section aria-labelledby="aumento-t" className="space-y-3">
          <h2 id="aumento-t" className="text-h3">Aumento na conta</h2>
          {rises.map((r) => {
            const bothOpen = !isExpired(byId.get(r.fromId)!, now) && !isExpired(byId.get(r.toId)!, now)
            return (
              <Alert key={`${r.fromId}-${r.toId}`} tone="warning">
                <p>
                  Sua conta de {r.provider} subiu <span className="font-bold tabular-nums">{r.pct}%</span> de {formatMonth(r.fromMonth)} para {formatMonth(r.toMonth)}.
                </p>
                {bothOpen && (
                  <Link to={`/comparar?a=${encodeURIComponent(r.fromId)}&b=${encodeURIComponent(r.toId)}`} className={LINK}>
                    Ver o que mudou
                  </Link>
                )}
              </Alert>
            )
          })}
        </section>
      )}

      {items.length > 0 && (
        <section aria-labelledby="lista-t" className="space-y-4">
          <h2 id="lista-t" className="text-h3">Suas contas</h2>
          <ul className="space-y-4">
            {items.map((item) => (
              <li key={item.id}>
                <BillCard
                  item={item}
                  now={now}
                  onRemove={(i) => {
                    removeItem(i.id)
                    afterChange(`Conta de ${i.provider}, ${formatMonth(i.month)}, removida da lista.`)
                  }}
                />
              </li>
            ))}
          </ul>

          {confirming ? (
            <Alert tone="danger" title="Apagar toda a lista?">
              <p>Isso tira todas as contas e protocolos deste aparelho. Os resultados continuam abrindo pelo link, até expirarem.</p>
              <div className="flex flex-wrap gap-3">
                <Button
                  onClick={() => {
                    clearHistory()
                    setConfirming(false)
                    afterChange('Lista apagada.')
                  }}
                >
                  Sim, apagar tudo
                </Button>
                <Button variant="secondary" onClick={() => setConfirming(false)}>Cancelar</Button>
              </div>
            </Alert>
          ) : (
            <Button variant="secondary" onClick={() => setConfirming(true)}>Apagar tudo</Button>
          )}
        </section>
      )}
    </Page>
  )
}
