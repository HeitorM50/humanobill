import { useQuery } from '@tanstack/react-query'
import { Alert, Button, ButtonLink, Card, Page, Spinner } from '../components/ui'
import { SeverityBadge } from '../components/result/SeverityBadge'
import { getStats } from '../lib/api'
import type { BillKind } from '../types/BillKind'
import type { KindStats } from '../types/KindStats'
import type { RuleStats } from '../types/RuleStats'
import type { Stats } from '../types/Stats'

const KIND_TITLE: Record<BillKind, string> = {
  telecom: 'Celular, internet e TV',
  energy: 'Conta de luz',
  bank_statement: 'Conta no banco',
  card_bill: 'Cartão de crédito',
}

const nf = new Intl.NumberFormat('pt-BR')
const faturas = (n: number) => `${nf.format(n)} ${n === 1 ? 'fatura enviada' : 'faturas enviadas'}`
const analises = (n: number) => `${nf.format(n)} ${n === 1 ? 'análise' : 'análises'}`

/** Barra horizontal: o número e o texto dizem tudo, a barra só reforça (e fica fora da leitura de tela). */
function RuleBar({ rule }: { rule: RuleStats }) {
  const fill = rule.severity === 'indevido' ? 'bg-danger' : 'bg-warning'
  return (
    <li className="space-y-2 border-b border-line py-4 last:border-b-0">
      <div className="flex flex-wrap items-center justify-between gap-x-3 gap-y-2">
        <h3 className="min-w-0 text-h4 break-words">{rule.title}</h3>
        <SeverityBadge severity={rule.severity} />
      </div>
      <div aria-hidden="true" className="h-3 overflow-hidden rounded-full bg-line">
        <div className={`h-full rounded-full ${fill}`} style={{ width: `${rule.percent}%` }} />
      </div>
      <p className="text-caption font-semibold">
        {rule.percent}% das faturas · {analises(rule.count)}
      </p>
    </li>
  )
}

function KindSection({ kind, rules }: { kind: KindStats; rules: RuleStats[] }) {
  const headingId = `painel-${kind.kind}`
  const empty = kind.total === null
  return (
    <section aria-labelledby={headingId} className="space-y-3">
      <div className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
        <h2 id={headingId} className="text-h3">{KIND_TITLE[kind.kind]}</h2>
        <p className="text-muted">{empty ? 'Poucos casos ainda' : faturas(kind.total!)}</p>
      </div>
      <Card>
        {empty || rules.length === 0 ? (
          <p className="text-muted">
            Poucos casos ainda. Quando houver pelo menos 5 casos parecidos, eles aparecem aqui.
          </p>
        ) : (
          <>
            <p className="text-caption text-muted">Parte das faturas enviadas que teve cada achado.</p>
            <ul>
              {rules.map((r) => (
                <RuleBar key={`${r.rule_id}-${r.severity}`} rule={r} />
              ))}
            </ul>
            {kind.has_hidden_findings && (
              <p className="border-t border-line pt-3 text-caption text-muted">
                Outros achados têm poucos casos ainda e não aparecem.
              </p>
            )}
          </>
        )}
      </Card>
    </section>
  )
}

function Content({ stats }: { stats: Stats }) {
  const nothing = stats.kinds.every((k) => k.total === null) && stats.rules.length === 0
  if (nothing) {
    return (
      <Card className="max-w-text space-y-4">
        <h2 className="text-h3">Ainda estamos juntando os primeiros casos</h2>
        <p>
          Para proteger a privacidade de quem enviou, só mostramos um número quando há pelo menos {stats.min_group} casos
          parecidos. Assim que isso acontecer, ele aparece aqui.
        </p>
        <ButtonLink to="/analisar">Enviar minha fatura</ButtonLink>
      </Card>
    )
  }
  return (
    <div className="space-y-8">
      {stats.kinds.map((k) => (
        <KindSection key={k.kind} kind={k} rules={stats.rules.filter((r) => r.kind === k.kind)} />
      ))}
    </div>
  )
}

export default function Panel() {
  const { data, isPending, isError, refetch } = useQuery({ queryKey: ['stats'], queryFn: getStats })
  return (
    <Page className="space-y-6">
      <header className="max-w-text space-y-3">
        <h1 className="text-h1">O que o HumanoBill já encontrou</h1>
        <p className="text-muted">Os problemas mais comuns nas faturas que as pessoas enviaram.</p>
      </header>
      <Alert tone="info" className="max-w-text">
        <p>Dados anônimos das faturas enviadas na demo pública. Grupos com menos de 5 casos não aparecem.</p>
      </Alert>
      {isPending && (
        <p role="status" className="flex items-center gap-2 text-muted">
          <Spinner /> Carregando os números…
        </p>
      )}
      {isError && (
        <Alert tone="danger" role="alert" title="Não consegui carregar os números." className="max-w-text">
          <p>Pode ser o servidor acordando. Tente de novo em instantes.</p>
          <Button variant="secondary" onClick={() => void refetch()}>Tentar de novo</Button>
        </Alert>
      )}
      {data && <Content stats={data} />}
    </Page>
  )
}
