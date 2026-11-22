import { useEffect, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link, useParams } from 'react-router'
import { Alert, Button, ButtonLink, Page } from '../components/ui'
import { ApiError, getAnalysis } from '../lib/api'
import { markExpired, recordAnalysis } from '../lib/history'
import { MotionConfig } from 'motion/react'
import { RecoverableCard } from '../components/result/RecoverableCard'
import { LiveDemo } from '../components/result/LiveDemo'
import { ResultBody } from '../components/result/ResultBody'
import { BillOverlay } from '../components/result/BillOverlay'

function Skeleton() {
  return (
    <div role="status" aria-label="Carregando resultado" className="animate-pulse">
      <div className="bg-line">
        <div className="mx-auto max-w-page space-y-3 px-5 py-8 md:px-10 md:py-14">
          <div className="h-4 w-1/3 rounded-control bg-surface/60" />
          <div className="h-14 w-2/3 rounded-control bg-surface/60" />
          <div className="h-12 w-1/2 rounded-control bg-surface/60" />
        </div>
      </div>
      <Page className="grid gap-6 md:grid-cols-2">
        {[0, 1].map((i) => <div key={i} className="h-48 rounded-card bg-line" />)}
      </Page>
    </div>
  )
}

function Message({ title, children }: { title: string; children?: React.ReactNode }) {
  return (
    <Page width="form">
      <div className="space-y-5">
        <h1 className="text-h1">{title}</h1>
        {children}
      </div>
    </Page>
  )
}

function CopyLink() {
  const [copied, setCopied] = useState(false)
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(window.location.href)
      setCopied(true)
      setTimeout(() => setCopied(false), 2500)
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

  // "Minhas contas neste aparelho": guarda a conta ao abrir (e de novo se ela mudar, ex.: uma resposta).
  useEffect(() => {
    if (data) recordAnalysis(data)
  }, [data])
  // Resultado que o servidor não tem mais: a lista passa a mostrar "expirado".
  const gone = error instanceof ApiError && (error.status === 404 || error.status === 410)
  useEffect(() => {
    if (gone) markExpired(id)
  }, [gone, id])

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
    return <Message title="Não consegui abrir este resultado"><Alert tone="danger" role="alert"><p>{error.message}</p></Alert></Message>
  }

  return (
    <MotionConfig reducedMotion="user">
      <RecoverableCard analysis={data} />

      {/* #51: a fatura com os itens marcados. Some quando não há imagem nem caixas. */}
      <div className="mx-auto w-full max-w-page px-5 pt-8 empty:hidden md:px-10 md:pt-14">
        <BillOverlay analysis={data} />
      </div>

      <ResultBody analysis={data}>
        <LiveDemo analysis={data} />

        <footer className="flex flex-wrap items-center gap-x-4 gap-y-3 border-t border-line pt-6">
          <p className="text-caption text-muted">Este resultado expira em 30 dias.</p>
          <CopyLink />
          <Link to="/" className="inline-flex min-h-11.5 items-center font-semibold text-accent underline underline-offset-4">Analisar outra conta</Link>
          <Link to={`/comparar?a=${encodeURIComponent(data.id)}`} className="inline-flex min-h-11.5 items-center font-semibold text-accent underline underline-offset-4">Comparar com outra fatura</Link>
        </footer>
      </ResultBody>
    </MotionConfig>
  )
}
