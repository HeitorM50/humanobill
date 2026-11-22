import { useEffect, useId, useRef, useState } from 'react'
import { useMutation } from '@tanstack/react-query'
import { useDropzone, type FileRejection } from 'react-dropzone'
import { Link, useNavigate } from 'react-router'
import { Alert, Button, Card, Icon, Page, Spinner } from '../components/ui'
import { ApiError, DEMO_SAMPLES, createDemoAnalysis, uploadAnalysis } from '../lib/api'
import { rememberUploadedBill } from '../components/result/billImage'

const MAX_BYTES = 10 * 1024 * 1024
const ACCEPT = {
  'application/pdf': ['.pdf'],
  'image/jpeg': ['.jpg', '.jpeg'],
  'image/png': ['.png'],
  'image/webp': ['.webp'],
}
const STAGES = ['Lendo sua fatura…', 'Conferindo item por item…', 'Pronto']

function rejectionMessage(r: FileRejection): string {
  const codes = r.errors.map((e) => e.code)
  if (codes.includes('file-too-large')) return 'O arquivo deve ter até 10 MB'
  if (codes.includes('too-many-files')) return 'Envie um arquivo por vez'
  return 'Envie um PDF ou uma foto (JPG, PNG)'
}

function Progress({ stage }: { stage: number }) {
  return (
    <Card>
      <ol className="space-y-2" aria-live="polite">
        {STAGES.map((s, i) => (
          <li key={s} className={`flex items-center gap-2 ${i <= stage ? 'font-semibold text-ink' : 'text-muted'}`}>
            {i < stage ? <Icon name="ok" size={16} className="text-ok" /> : i === stage ? <Spinner /> : <span aria-hidden="true" className="size-4 shrink-0 rounded-full border-2 border-line" />}
            {s}
          </li>
        ))}
      </ol>
    </Card>
  )
}

export default function Analyze() {
  const navigate = useNavigate()
  const consentId = useId()
  const cameraRef = useRef<HTMLInputElement>(null)
  const [file, setFile] = useState<File | null>(null)
  const [fileError, setFileError] = useState<string | null>(null)
  const [consent, setConsent] = useState(false)
  const [stage, setStage] = useState(0)

  const upload = useMutation({
    mutationFn: uploadAnalysis,
    onMutate: () => setStage(0),
    onSuccess: (a, sent) => {
      // A imagem enviada fica só na memória da aba, para a marcação no resultado.
      rememberUploadedBill(a.id, sent)
      setStage(2)
      setTimeout(() => navigate(`/r/${a.id}`), 400)
    },
  })
  const demo = useMutation({ mutationFn: createDemoAnalysis, onSuccess: (a) => navigate(`/r/${a.id}`) })

  // Etapas simuladas por tempo enquanto a requisição roda.
  useEffect(() => {
    if (!upload.isPending) return
    const t = setTimeout(() => setStage(1), 3000)
    return () => clearTimeout(t)
  }, [upload.isPending])

  const pick = (accepted: File[], rejected: FileRejection[]) => {
    upload.reset()
    if (rejected.length) {
      setFile(null)
      setFileError(rejectionMessage(rejected[0]))
    } else if (accepted[0]) {
      setFile(accepted[0])
      setFileError(null)
    }
  }
  const { getRootProps, getInputProps, isDragActive, open } = useDropzone({
    accept: ACCEPT,
    maxSize: MAX_BYTES,
    multiple: false,
    noClick: true,
    onDrop: pick,
  })

  const onCamera = (e: React.ChangeEvent<HTMLInputElement>) => {
    const f = e.target.files?.[0]
    e.target.value = ''
    if (!f) return
    if (f.size > MAX_BYTES) pick([], [{ file: f as FileRejection['file'], errors: [{ code: 'file-too-large', message: '' }] }])
    else pick([f], [])
  }

  const busy = upload.isPending || upload.isSuccess
  const canSubmit = !!file && consent && !busy
  const err = upload.error instanceof ApiError ? upload.error : null
  const extractionFailed = err?.code === 'extraction_failed' || err?.code === 'extraction_unavailable'

  return (
    <Page width="form" className="space-y-6">
      <h1 className="text-h1">Enviar minha fatura</h1>

      <div
        {...getRootProps({
          className: `rounded-card border-2 border-dashed p-6 text-center transition-colors duration-150 ${
            isDragActive ? 'border-accent bg-accent-soft' : 'border-muted bg-surface'
          }`,
        })}
      >
        <input {...getInputProps({ 'aria-label': 'Arquivo da fatura' })} />
        <p>Arraste o arquivo aqui ou escolha uma opção.</p>
        <div className="mt-4 flex flex-col justify-center gap-3 sm:flex-row">
          <Button onClick={open} disabled={busy}>Escolher arquivo</Button>
          <Button variant="secondary" onClick={() => cameraRef.current?.click()} disabled={busy}>Tirar foto</Button>
        </div>
        <input ref={cameraRef} type="file" accept="image/*" capture="environment" className="hidden" onChange={onCamera} tabIndex={-1} aria-hidden="true" />
        <p className="mt-4 text-caption text-muted">PDF, JPG, PNG ou WEBP, até 10 MB.</p>
        {file && <p className="mt-3 font-semibold break-all">Arquivo: {file.name}</p>}
        {fileError && (
          <p role="alert" className="mt-3 inline-flex items-center gap-2 font-semibold text-danger">
            <Icon name="danger" size={16} />
            {fileError}
          </p>
        )}
      </div>

      <Alert tone="warning">
        <p>
          Demo em ambiente gratuito: os dados podem ser processados pelo provedor de IA. Prefira as faturas de exemplo.{' '}
          <Link to="/privacidade" className="font-semibold underline underline-offset-4">Saiba mais</Link>
        </p>
        <div className="flex min-h-11.5 items-center gap-3">
          <input id={consentId} type="checkbox" checked={consent} onChange={(e) => setConsent(e.target.checked)} className="size-5.5 shrink-0 accent-accent" />
          <label htmlFor={consentId} className="flex min-h-11.5 flex-1 cursor-pointer items-center font-semibold">Entendi e concordo</label>
        </div>
      </Alert>

      {busy ? (
        <Progress stage={stage} />
      ) : (
        <Button className="w-full" disabled={!canSubmit} onClick={() => file && upload.mutate(file)}>Analisar</Button>
      )}

      {err && (
        <Alert tone="danger" role="alert">
          <p className="font-semibold">{err.message}</p>
          {extractionFailed && (
            <Button variant="secondary" disabled={demo.isPending} onClick={() => demo.mutate(DEMO_SAMPLES[0].id)}>
              Usar uma fatura de exemplo
            </Button>
          )}
        </Alert>
      )}
    </Page>
  )
}
