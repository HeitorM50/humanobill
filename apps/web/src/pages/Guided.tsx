import { useEffect } from 'react'
import { useQuery } from '@tanstack/react-query'
import { MotionConfig } from 'motion/react'
import { useParams } from 'react-router'
import { GuidedFlow } from '../components/guided/GuidedFlow'
import { Alert, ButtonLink, Page } from '../components/ui'
import { ApiError, getAnalysis } from '../lib/api'
import { markExpired } from '../lib/history'

function Message({ title, children }: { title: string; children?: React.ReactNode }) {
  return (
    <Page width="form" className="space-y-5">
      <h1 className="text-h1">{title}</h1>
      {children}
    </Page>
  )
}

const RESTART = (
  <ButtonLink to="/guiado" className="min-h-14! w-full text-h4 sm:w-auto">Começar de novo</ButtonLink>
)

/** Abre a análise e entrega ao passo a passo. A análise é a mesma da versão completa (mesmo cache). */
function GuidedLoader({ id }: { id: string }) {
  const { data, error, isPending } = useQuery({ queryKey: ['analysis', id], queryFn: () => getAnalysis(id) })
  // Resultado que o servidor não tem mais: a lista "Minhas contas" passa a mostrar "expirado".
  const gone = error instanceof ApiError && (error.status === 404 || error.status === 410)
  useEffect(() => {
    if (gone) markExpired(id)
  }, [gone, id])

  if (isPending) {
    return (
      <Page width="form">
        <p role="status" className="text-h4 text-muted">Abrindo a sua fatura…</p>
      </Page>
    )
  }
  if (error) {
    const status = error instanceof ApiError ? error.status : 0
    if (status === 404) return <Message title="Resultado não encontrado">{RESTART}</Message>
    if (status === 410) return <Message title="Este resultado expirou. Analise de novo.">{RESTART}</Message>
    return (
      <Message title="Não consegui abrir este resultado">
        <Alert tone="danger" role="alert"><p>{error.message}</p></Alert>
        {RESTART}
      </Message>
    )
  }
  return <GuidedFlow analysis={data} />
}

/**
 * RF22 — modo passo a passo. `/guiado` começa pelo envio da fatura; `/guiado/:id` continua
 * uma análise que já existe. A versão completa (`/analisar`, `/r/:id`) não muda.
 */
export default function Guided() {
  const { id } = useParams()
  return (
    <MotionConfig reducedMotion="user">
      {id ? <GuidedLoader key={id} id={id} /> : <GuidedFlow key="novo" analysis={null} />}
    </MotionConfig>
  )
}
