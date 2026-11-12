import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link, useParams } from 'react-router'
import { Button, ButtonLink } from '../components/ui'
import { ApiError, getAnalysis } from '../lib/api'
import { formatBRL, formatMonth } from '../lib/format'
import { MotionConfig } from 'motion/react'
import { RecoverableCard } from '../components/result/RecoverableCard'
import { CorrectionDrawer } from '../components/result/CorrectionDrawer'
import { LiveDemo } from '../components/result/LiveDemo'
import { ResultBody } from '../components/result/ResultBody'

function Skeleton() {
  return (
    <div role="status" aria-label="Carregando resultado" className="animate-pulse space-y-4">
      <div className="h-8 w-2/3 rounded bg-line" />
      <div className="h-12 w-1/2 rounded bg-line" />
      {[0, 1, 2].map((i) => <div key={i} className="h-24 rounded-card bg-line" />)}
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

  const { bill } = data

  return (
    <MotionConfig reducedMotion="user">
    <div className="space-y-8">
      <header className="space-y-1">
        <p className="text-muted">{bill.provider_name} · {formatMonth(bill.reference_month)}</p>
        <p className="text-5xl font-bold tracking-tight tabular-nums">{formatBRL(bill.total_cents)}</p>
        <p className="text-sm text-muted">Total da fatura</p>
        <CorrectionDrawer analysis={data} />
      </header>

      <RecoverableCard analysis={data} />

      <ResultBody analysis={data} />

      <LiveDemo analysis={data} />

      <footer className="flex flex-wrap items-center gap-3">
        <CopyLink />
        <Link to="/" className="inline-flex min-h-11 items-center underline">Analisar outra conta</Link>
      </footer>
    </div>
    </MotionConfig>
  )
}
