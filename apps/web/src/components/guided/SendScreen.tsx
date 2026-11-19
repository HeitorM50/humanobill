import { useId, useRef, useState } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Link, useNavigate } from 'react-router'
import { ApiError, DEMO_GROUPS, DEMO_SAMPLES, createDemoAnalysis, uploadAnalysis, uploadSample } from '../../lib/api'
import { formatBRL, formatMonth } from '../../lib/format'
import type { Analysis } from '../../types/Analysis'
import { Alert, Button, Card, Icon, Spinner } from '../ui'

// Mesmos limites da versão completa (Analyze.tsx).
const MAX_BYTES = 10 * 1024 * 1024
const ACCEPT = '.pdf,.jpg,.jpeg,.png,.webp,application/pdf,image/jpeg,image/png,image/webp'
const ACCEPTED_TYPES = ['application/pdf', 'image/jpeg', 'image/png', 'image/webp']

/** Botão largo de escolha, com alvo de toque grande. */
const CHOICE = 'min-h-14! w-full text-h4'

function fileProblem(f: File): string | null {
  if (f.size > MAX_BYTES) return 'O arquivo deve ter até 10 MB'
  if (!ACCEPTED_TYPES.includes(f.type)) return 'Envie um PDF ou uma foto (JPG, PNG)'
  return null
}

type Source = 'exemplo' | 'arquivo' | null

/** Etapa 1 com a fatura já enviada: só confirma o que foi lido e deixa trocar. */
export function SentSummary({ analysis }: { analysis: Analysis }) {
  const { bill } = analysis
  return (
    <>
      <Card className="space-y-1">
        <p className="text-h4 font-bold">{bill.provider_name}</p>
        <p className="text-muted">Fatura de {formatMonth(bill.reference_month)}</p>
        <p className="text-h3 font-bold">{formatBRL(bill.total_cents)}</p>
      </Card>
      <p className="max-w-text text-h4">Já li a sua fatura. Vamos conferir juntos, um passo de cada vez.</p>
      <p>
        <Link to="/guiado" className="inline-flex min-h-12 items-center font-semibold text-accent underline underline-offset-4">
          Enviar outra fatura
        </Link>
      </p>
    </>
  )
}

/** Etapa 1 sem fatura: usar um exemplo ou enviar um arquivo (com consentimento). */
export function SendChoice() {
  const navigate = useNavigate()
  const qc = useQueryClient()
  const consentId = useId()
  const fileInput = useRef<HTMLInputElement>(null)
  const cameraInput = useRef<HTMLInputElement>(null)
  const [source, setSource] = useState<Source>(null)
  const [file, setFile] = useState<File | null>(null)
  const [fileError, setFileError] = useState<string | null>(null)
  const [consent, setConsent] = useState(false)
  const [liveRead, setLiveRead] = useState(false)
  const liveId = useId()

  // A análise vai para o cache antes de trocar de tela, para a próxima abrir sem piscar.
  const done = (a: Analysis) => {
    qc.setQueryData(['analysis', a.id], a)
    navigate(`/guiado/${a.id}`)
  }
  const demo = useMutation({ mutationFn: createDemoAnalysis, onSuccess: done })
  const upload = useMutation({ mutationFn: uploadAnalysis, onSuccess: done })
  // Mesmo exemplo, mas reenviando o PDF para a IA ler ao vivo (como na versão completa).
  const liveDemo = useMutation({ mutationFn: uploadSample, onSuccess: done })

  const pick = (f: File | undefined) => {
    upload.reset()
    if (!f) return
    const problem = fileProblem(f)
    setFile(problem ? null : f)
    setFileError(problem)
  }

  const err = upload.error instanceof ApiError ? upload.error : null
  const extractionFailed = err?.code === 'extraction_failed' || err?.code === 'extraction_unavailable'
  const busy = upload.isPending || upload.isSuccess
  const exampleBusy = demo.isPending || liveDemo.isPending || liveDemo.isSuccess

  if (source === null) {
    return (
      <>
        <p className="max-w-text text-h4">Como você quer começar?</p>
        <div className="space-y-3">
          <Button className={CHOICE} onClick={() => setSource('arquivo')}>Enviar a minha fatura</Button>
          <Button variant="secondary" className={CHOICE} onClick={() => setSource('exemplo')}>Usar uma fatura de exemplo</Button>
        </div>
        <p className="flex items-center gap-2 text-caption text-muted">
          <Icon name="lock" size={16} />
          Seus dados pessoais não são guardados.
        </p>
      </>
    )
  }

  if (source === 'exemplo') {
    return (
      <>
        <p className="max-w-text text-h4">Escolha um exemplo. As faturas são de mentira, com empresas inventadas.</p>
        <label
          htmlFor={liveId}
          className="flex min-h-14 cursor-pointer items-start gap-3 rounded-card border border-line bg-surface p-4 has-checked:border-accent has-checked:bg-accent-soft"
        >
          <input
            id={liveId}
            type="checkbox"
            checked={liveRead}
            disabled={exampleBusy}
            onChange={(e) => setLiveRead(e.target.checked)}
            className="mt-1 size-6 shrink-0 accent-accent"
          />
          <span className="min-w-0 flex-1">
            <span className="block font-semibold">Quero ver a IA lendo o PDF</span>
            <span className="block text-muted">A fatura de exemplo (sem dados reais) é enviada para a IA. Pode levar alguns segundos.</span>
          </span>
        </label>
        {DEMO_GROUPS.map((g, gi) => (
          <div key={g} role="group" aria-labelledby={`guiado-grupo-${gi}`} className="space-y-3">
            <h2 id={`guiado-grupo-${gi}`} className="text-h3">{g}</h2>
            <ul className="space-y-3">
              {DEMO_SAMPLES.filter((s) => s.group === g).map((s) => (
                <li key={s.id}>
                  <Button variant="secondary" className={CHOICE} disabled={exampleBusy} onClick={() => (liveRead ? liveDemo.mutate(s.id) : demo.mutate(s.id))}>
                    <span className="min-w-0 flex-1 text-left break-words">{s.title}</span>
                  </Button>
                </li>
              ))}
            </ul>
          </div>
        ))}
        {demo.isPending && <p role="status" className="text-muted">Abrindo o exemplo…</p>}
        {liveDemo.isPending && (
          <p role="status" className="flex items-center gap-2 font-semibold">
            <Spinner />
            Lendo a fatura com IA… pode levar alguns segundos.
          </p>
        )}
        {demo.isError && (
          <Alert tone="danger" role="alert">
            <p>{demo.error.message}</p>
          </Alert>
        )}
        {liveDemo.isError && (
          <Alert tone="danger" role="alert">
            <p>{liveDemo.error.message.replace(/[.\s]+$/, '')}. Desmarque a leitura com IA para usar o exemplo pronto.</p>
          </Alert>
        )}
        <Button variant="ghost" className="min-h-12" disabled={exampleBusy} onClick={() => setSource(null)}>Escolher outro jeito</Button>
      </>
    )
  }

  return (
    <>
      <p className="max-w-text text-h4">Escolha o arquivo da fatura. Pode ser PDF ou foto.</p>
      <div className="space-y-3">
        <Button className={CHOICE} disabled={busy} onClick={() => fileInput.current?.click()}>Escolher arquivo</Button>
        <Button variant="secondary" className={CHOICE} disabled={busy} onClick={() => cameraInput.current?.click()}>Tirar foto</Button>
      </div>
      <input
        ref={fileInput}
        type="file"
        accept={ACCEPT}
        aria-label="Arquivo da fatura"
        className="hidden"
        tabIndex={-1}
        onChange={(e) => {
          pick(e.target.files?.[0])
          e.target.value = ''
        }}
      />
      <input
        ref={cameraInput}
        type="file"
        accept="image/*"
        capture="environment"
        aria-label="Foto da fatura"
        className="hidden"
        tabIndex={-1}
        onChange={(e) => {
          pick(e.target.files?.[0])
          e.target.value = ''
        }}
      />
      <p className="text-caption text-muted">PDF, JPG, PNG ou WEBP, até 10 MB.</p>
      {file && <p className="font-semibold break-all">Arquivo: {file.name}</p>}
      {fileError && (
        <p role="alert" className="inline-flex items-center gap-2 font-semibold text-danger">
          <Icon name="danger" size={16} />
          {fileError}
        </p>
      )}

      <Alert tone="warning">
        <p>
          Demo em ambiente gratuito: os dados podem ser processados pelo provedor de IA. Prefira as faturas de exemplo.{' '}
          <Link to="/privacidade" className="font-semibold underline underline-offset-4">Saiba mais</Link>
        </p>
        <div className="flex min-h-12 items-center gap-3">
          <input id={consentId} type="checkbox" checked={consent} onChange={(e) => setConsent(e.target.checked)} className="size-6 shrink-0 accent-accent" />
          <label htmlFor={consentId} className="flex min-h-12 flex-1 cursor-pointer items-center font-semibold">Entendi e concordo</label>
        </div>
      </Alert>

      <Button className={CHOICE} disabled={!file || !consent || busy} onClick={() => file && upload.mutate(file)}>Analisar</Button>
      {busy && (
        <p role="status" className="flex items-center gap-2 font-semibold">
          <Spinner />
          Lendo sua fatura… pode levar alguns segundos.
        </p>
      )}
      {err && (
        <Alert tone="danger" role="alert">
          <p className="font-semibold">{err.message}</p>
          {extractionFailed && (
            <Button variant="secondary" className="min-h-14!" onClick={() => setSource('exemplo')}>Usar uma fatura de exemplo</Button>
          )}
        </Alert>
      )}
      <Button variant="ghost" className="min-h-12" disabled={busy} onClick={() => setSource(null)}>Escolher outro jeito</Button>
    </>
  )
}
