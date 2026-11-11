import { useEffect, useId, useRef, useState } from 'react'
import { useMutation } from '@tanstack/react-query'
import { useDropzone, type FileRejection } from 'react-dropzone'
import { Link, useNavigate } from 'react-router'
import { Button, Card } from '../components/ui'
import { ApiError, DEMO_SAMPLES, createDemoAnalysis, uploadAnalysis } from '../lib/api'

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
          <li key={s} className={i <= stage ? 'font-semibold text-slate-900' : 'text-slate-500'}>
            <span aria-hidden="true">{i < stage ? '✓ ' : i === stage ? '… ' : '○ '}</span>
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
    onSuccess: (a) => {
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
    <div className="space-y-6">
      <h1 className="text-3xl font-bold tracking-tight">Enviar minha fatura</h1>

      <div
        {...getRootProps({
          className: `rounded-2xl border-2 border-dashed p-6 text-center ${isDragActive ? 'border-emerald-600 bg-emerald-50' : 'border-slate-300 bg-white'}`,
        })}
      >
        <input {...getInputProps({ 'aria-label': 'Arquivo da fatura' })} />
        <p className="text-slate-700">Arraste o arquivo aqui ou escolha uma opção.</p>
        <div className="mt-4 flex flex-col justify-center gap-3 sm:flex-row">
          <Button onClick={open} disabled={busy}>Escolher arquivo</Button>
          <Button variant="secondary" onClick={() => cameraRef.current?.click()} disabled={busy}>Tirar foto</Button>
        </div>
        <input ref={cameraRef} type="file" accept="image/*" capture="environment" className="hidden" onChange={onCamera} tabIndex={-1} aria-hidden="true" />
        <p className="mt-3 text-sm text-slate-600">PDF, JPG, PNG ou WEBP, até 10 MB.</p>
        {file && <p className="mt-3 font-medium break-all">Arquivo: {file.name}</p>}
        {fileError && <p role="alert" className="mt-3 font-medium text-red-800">{fileError}</p>}
      </div>

      <Card className="space-y-3 border-amber-300 bg-amber-50">
        <p className="text-amber-950">
          Demo em ambiente gratuito: os dados podem ser processados pelo provedor de IA. Prefira as faturas de exemplo.{' '}
          <Link to="/privacidade" className="font-semibold underline">Saiba mais</Link>
        </p>
        <div className="flex items-start gap-3">
          <input id={consentId} type="checkbox" checked={consent} onChange={(e) => setConsent(e.target.checked)} className="mt-1 size-6 shrink-0 accent-emerald-700" />
          <label htmlFor={consentId} className="min-h-11 content-center font-medium">Entendi e concordo</label>
        </div>
      </Card>

      {busy ? (
        <Progress stage={stage} />
      ) : (
        <Button className="w-full" disabled={!canSubmit} onClick={() => file && upload.mutate(file)}>Analisar</Button>
      )}

      {err && (
        <div role="alert" className="space-y-3 rounded-xl border border-red-300 bg-red-50 p-4 text-red-900">
          <p className="font-medium">{err.message}</p>
          {extractionFailed && (
            <Button variant="secondary" disabled={demo.isPending} onClick={() => demo.mutate(DEMO_SAMPLES[0].id)}>
              Usar uma fatura de exemplo
            </Button>
          )}
        </div>
      )}
    </div>
  )
}
