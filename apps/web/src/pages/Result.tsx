import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link, useParams } from 'react-router'
import { Button, ButtonLink, Card } from '../components/ui'
import { ApiError, getAnalysis } from '../lib/api'
import { formatBRL, formatMonth } from '../lib/format'
import type { Severity } from '../types/Severity'

const SEVERITY: Record<Severity, { icon: string; label: string; cls: string }> = {
  indevido: { icon: '✕', label: 'Indevido', cls: 'bg-red-100 text-red-900' },
  suspeito: { icon: '!', label: 'Suspeito', cls: 'bg-amber-100 text-amber-900' },
  ok: { icon: '✓', label: 'Conferido', cls: 'bg-emerald-100 text-emerald-900' },
}

function Skeleton() {
  return (
    <div role="status" aria-label="Carregando resultado" className="animate-pulse space-y-4">
      <div className="h-8 w-2/3 rounded bg-slate-200" />
      <div className="h-12 w-1/2 rounded bg-slate-200" />
      {[0, 1, 2].map((i) => <div key={i} className="h-24 rounded-2xl bg-slate-200" />)}
    </div>
  )
}

function Message({ title, children }: { title: string; children?: React.ReactNode }) {
  return (
    <div className="space-y-4 py-8">
      <h1 className="text-2xl font-bold">{title}</h1>
      {children}
    </div>
  )
}

function CopyLink() {
  const [copied, setCopied] = useState(false)
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(window.location.href)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch {
      /* sem permissão de área de transferência */
    }
  }
  return (
    <Button variant="secondary" onClick={copy}>
      {copied ? 'Copiado!' : 'Copiar link'}
      <span role="status" className="sr-only">{copied ? 'Link copiado' : ''}</span>
    </Button>
  )
}

export default function Result() {
  const { id = '' } = useParams()
  const { data, error, isPending } = useQuery({ queryKey: ['analysis', id], queryFn: () => getAnalysis(id) })

  if (isPending) return <Skeleton />
  if (error) {
    const status = error instanceof ApiError ? error.status : 0
    if (status === 404)
      return <Message title="Resultado não encontrado"><ButtonLink to="/">Voltar ao início</ButtonLink></Message>
    if (status === 410)
      return (
        <Message title="Este resultado expirou. Analise de novo.">
          <ButtonLink to="/analisar">Analisar de novo</ButtonLink>
        </Message>
      )
    return <Message title="Não consegui abrir este resultado"><p>{error.message}</p></Message>
  }

  const { bill, findings, questions } = data
  const checking = findings.length === 0 && questions.length === 0

  return (
    <div className="space-y-8">
      <header className="space-y-1">
        <p className="text-slate-700">{bill.provider_name} · {formatMonth(bill.reference_month)}</p>
        <p className="text-5xl font-bold tracking-tight tabular-nums">{formatBRL(bill.total_cents)}</p>
        <p className="text-sm text-slate-600">Total da fatura</p>
      </header>

      <section aria-labelledby="prob-t" className="space-y-3">
        <h2 id="prob-t" className="text-2xl font-bold">Problemas encontrados</h2>
        {/* TODO(S2): mostrar perguntas rápidas, "Tudo certo" quando as regras existirem e cartões completos de achado. */}
        {checking ? (
          <Card><p>Ainda conferindo as regras desta conta.</p></Card>
        ) : (
          findings.map((f) => {
            const s = SEVERITY[f.severity]
            return (
              <Card key={f.rule_id + f.item_refs.join()}>
                <span className={`inline-flex items-center gap-1 rounded-full px-3 py-1 text-sm font-semibold ${s.cls}`}>
                  <span aria-hidden="true">{s.icon}</span> {s.label}
                </span>
                <h3 className="mt-2 text-lg font-semibold">{f.title}</h3>
                <p className="text-slate-700">{f.explanation}</p>
                <p className="mt-1 font-semibold tabular-nums">{formatBRL(f.amount_cents)}</p>
              </Card>
            )
          })
        )}
      </section>

      <section aria-labelledby="itens-t" className="space-y-3">
        <h2 id="itens-t" className="text-2xl font-bold">Sua conta explicada</h2>
        <ul className="space-y-3">
          {bill.items.map((it) => (
            <li key={it.id}>
              <Card>
                <div className="flex items-start justify-between gap-3">
                  <h3 className="font-semibold">{it.description}</h3>
                  <span className="font-semibold whitespace-nowrap tabular-nums">{formatBRL(it.amount_cents)}</span>
                </div>
                <p className="mt-1 text-slate-700">{it.plain_explanation}</p>
                <p className="mt-2 font-mono text-xs break-words text-slate-600">{it.original_text}</p>
              </Card>
            </li>
          ))}
        </ul>
      </section>

      <footer className="flex flex-wrap items-center gap-3">
        <CopyLink />
        <Link to="/" className="inline-flex min-h-11 items-center underline">Analisar outra conta</Link>
      </footer>
    </div>
  )
}
