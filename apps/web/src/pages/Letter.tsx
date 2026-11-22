import { useQuery } from '@tanstack/react-query'
import { Link, useParams, useSearchParams } from 'react-router'
import { Alert, Button, ButtonLink, Page, Spinner } from '../components/ui'
import { ApiError, createDispute, getAnalysis } from '../lib/api'
import { formatMonth } from '../lib/format'
import { formalLetterText } from '../lib/letter'

/** Mesma regra do servidor e da DisputeSection: só dígitos e hífen, até 30 caracteres. */
const PROTOCOL_RE = /^[0-9-]{0,30}$/

/** Linha em branco para preencher à mão depois de imprimir. */
function BlankLine({ label, width = 'w-72' }: { label: string; width?: string }) {
  return (
    <p className="carta-linha flex items-end gap-2">
      <span className="shrink-0">{label}</span>
      <span aria-hidden="true" className={`inline-block h-6 max-w-full min-w-24 flex-1 border-b border-ink sm:max-w-none ${width}`} />
    </p>
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

/**
 * Carta formal em A4 (RF11): o texto da contestação num layout de papel, pronto para
 * "Salvar como PDF" pelo navegador. Não preenche nome, CPF nem endereço: só linhas em branco.
 */
export default function Letter() {
  const { id = '' } = useParams()
  const [params] = useSearchParams()
  const items = params.getAll('item')
  const rawProtocol = (params.get('protocolo') ?? '').trim()
  const protocol = PROTOCOL_RE.test(rawProtocol) ? rawProtocol : ''

  const analysis = useQuery({ queryKey: ['analysis', id], queryFn: () => getAnalysis(id), enabled: items.length > 0 })
  const dispute = useQuery({
    queryKey: ['dispute-letter', id, [...items].sort(), protocol],
    queryFn: () => createDispute(id, { finding_ids: items, ...(protocol ? { protocol } : {}) }),
    enabled: items.length > 0,
  })

  const back = `/r/${encodeURIComponent(id)}`

  if (items.length === 0)
    return (
      <Message title="Nenhum problema escolhido para a carta">
        <p className="max-w-text">Volte ao resultado, escolha o que contestar e toque em "Carta formal (PDF)".</p>
        <ButtonLink to={back}>Voltar ao resultado</ButtonLink>
      </Message>
    )

  const error = analysis.error ?? dispute.error
  if (error) {
    const status = error instanceof ApiError ? error.status : 0
    if (status === 404) return <Message title="Resultado não encontrado"><ButtonLink to="/">Voltar ao início</ButtonLink></Message>
    if (status === 410)
      return (
        <Message title="Este resultado expirou. Analise de novo.">
          <ButtonLink to="/analisar">Analisar de novo</ButtonLink>
        </Message>
      )
    return (
      <Message title="Não consegui montar a carta">
        <Alert tone="danger" role="alert"><p>{error.message}</p></Alert>
        <ButtonLink to={back}>Voltar ao resultado</ButtonLink>
      </Message>
    )
  }

  if (!analysis.data || !dispute.data)
    return (
      <Page width="form">
        <p role="status" className="flex items-center gap-2 font-semibold">
          <Spinner />
          Montando a sua carta…
        </p>
      </Page>
    )

  const { bill } = analysis.data
  const text = formalLetterText(dispute.data.text)

  return (
    <Page width="form" className="carta-pagina space-y-6">
      <div className="carta-ajuda max-w-text space-y-3">
        <h1 className="text-h2">Carta formal</h1>
        <p className="text-muted">
          Toque em "Baixar carta (PDF)". Na janela que abrir, escolha <strong className="text-ink">Salvar como PDF</strong> (ou "Imprimir em PDF") como destino. Depois é só preencher o local, a data e assinar.
        </p>
        <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
          <Button onClick={() => window.print()}>Baixar carta (PDF)</Button>
          <Link to={back} className="inline-flex min-h-11.5 items-center font-semibold text-accent underline underline-offset-4">
            Voltar ao resultado
          </Link>
        </div>
      </div>

      <article aria-label="Carta de contestação" className="carta-folha space-y-6 rounded-card border border-line bg-surface p-6 text-ink md:p-10">
        <BlankLine label="Local e data:" />

        <p className="carta-linha">
          À <strong>{bill.provider_name}</strong>
        </p>

        <p className="carta-linha">
          <strong>Assunto:</strong> Contestação de cobranças da fatura de {formatMonth(bill.reference_month)}
          {protocol && <> (protocolo nº {protocol})</>}
        </p>

        <div className="carta-corpo break-words whitespace-pre-wrap">{text}</div>

        <div className="carta-assinatura space-y-6 pt-8">
          <BlankLine label="Nome:" width="w-80" />
          <BlankLine label="Assinatura:" width="w-80" />
          <p className="carta-linha">Protocolo nº: {protocol ? <strong>{protocol}</strong> : <span aria-hidden="true" className="inline-block h-6 w-48 border-b border-ink align-bottom" />}</p>
        </div>
      </article>
    </Page>
  )
}
