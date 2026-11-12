import { useId, useRef, useState } from 'react'
import { useMutation } from '@tanstack/react-query'
import { createDispute } from '../../lib/api'
import { formatBRL } from '../../lib/format'
import type { Analysis } from '../../types/Analysis'
import type { Dispute } from '../../types/Dispute'
import { Button, Card } from '../ui'
import { DISPUTE_SECTION_ID } from './RecoverableCard'

const PROTOCOL_RE = /^[\p{L}\p{N}\-./ ]*$/u

function ExternalIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M14 4h6v6M20 4l-9 9M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5" />
    </svg>
  )
}

function CopyText({ text }: { text: string }) {
  const ref = useRef<HTMLDivElement>(null)
  const [state, setState] = useState<'idle' | 'copied' | 'manual'>('idle')

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(text)
      setState('copied')
      setTimeout(() => setState('idle'), 2500)
    } catch {
      // Sem permissão ou sem API: seleciona o texto para o usuário copiar na mão.
      const el = ref.current
      if (el) {
        const range = document.createRange()
        range.selectNodeContents(el)
        const sel = window.getSelection()
        sel?.removeAllRanges()
        sel?.addRange(range)
        el.focus()
      }
      setState('manual')
    }
  }

  return (
    <div className="space-y-3">
      <div
        ref={ref}
        tabIndex={0}
        role="region"
        aria-label="Texto da contestação"
        className="rounded-control border border-line bg-bg p-4 leading-relaxed whitespace-pre-wrap break-words text-ink"
      >
        {text}
      </div>
      <div className="flex flex-wrap items-center gap-3">
        <Button onClick={copy}>{state === 'copied' ? 'Copiado!' : 'Copiar'}</Button>
        <p role="status" aria-live="polite" className="text-sm font-medium text-ok">
          {state === 'copied' && 'Copiado! Agora é só colar.'}
          {state === 'manual' && <span className="text-warning">Selecione e copie</span>}
        </p>
      </div>
    </div>
  )
}

function Roadmap({ dispute }: { dispute: Dispute }) {
  return (
    <section aria-labelledby="onde-t" className="space-y-4">
      <h3 id="onde-t" className="text-xl font-bold">Onde reclamar</h3>
      {dispute.deadlines.length > 0 && (
        <ul className="space-y-2 rounded-control border border-warning-line bg-warning-soft p-4">
          {dispute.deadlines.map((d) => (
            <li key={d} className="flex items-start gap-2 font-semibold text-ink">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" className="mt-0.5 shrink-0 text-warning">
                <circle cx="12" cy="12" r="9" />
                <path d="M12 7v5l3 2" />
              </svg>
              {d}
            </li>
          ))}
        </ul>
      )}
      <ol className="space-y-3">
        {dispute.steps.map((s, i) => (
          <li key={s.title} className="flex gap-3 rounded-control border border-line bg-surface p-4">
            <span aria-hidden="true" className="flex size-8 shrink-0 items-center justify-center rounded-full bg-accent font-bold text-white">
              {i + 1}
            </span>
            <div className="min-w-0">
              <p className="font-semibold">{s.title}</p>
              <p className="text-muted">{s.description}</p>
              {s.url && (
                <a
                  href={s.url}
                  target="_blank"
                  rel="noopener noreferrer"
                  aria-label={`${s.title} (abre em nova aba)`}
                  className="mt-1 inline-flex min-h-11 items-center gap-1.5 font-semibold text-accent underline underline-offset-2 hover:text-accent-strong"
                >
                  Abrir site
                  <ExternalIcon />
                </a>
              )}
            </div>
          </li>
        ))}
      </ol>
    </section>
  )
}

export function DisputeSection({ analysis, selectedIds }: { analysis: Analysis; selectedIds: string[] }) {
  const [protocol, setProtocol] = useState('')
  const [done, setDone] = useState<{ dispute: Dispute; key: string } | null>(null)
  const protoId = useId()
  const errId = useId()

  const trimmed = protocol.trim()
  const protocolInvalid = trimmed.length > 40 || !PROTOCOL_RE.test(trimmed)
  const key = JSON.stringify([[...selectedIds].sort(), trimmed, analysis.paid])

  const mutation = useMutation({
    mutationFn: (k: string) =>
      createDispute(analysis.id, { finding_ids: selectedIds, ...(trimmed ? { protocol: trimmed } : {}) }).then((dispute) => ({ dispute, key: k })),
    onSuccess: setDone,
  })

  const total = analysis.findings.filter((f) => selectedIds.includes(f.id)).reduce((t, f) => t + f.amount_cents, 0)
  const n = selectedIds.length
  const stale = done != null && done.key !== key

  return (
    <section id={DISPUTE_SECTION_ID} aria-labelledby="contestar-t" className="scroll-mt-4 space-y-3">
      <div>
        <h2 id="contestar-t" tabIndex={-1} className="text-2xl font-bold outline-none">Contestar</h2>
        <p className="text-muted">Escolha os problemas acima e a gente escreve o texto pronto.</p>
      </div>
      <Card className="space-y-4">
        <p className="text-lg font-semibold" aria-live="polite">
          {n === 0
            ? 'Nenhum problema selecionado'
            : `${n} ${n === 1 ? 'problema selecionado' : 'problemas selecionados'} · ${formatBRL(total)}`}
        </p>
        <div className="space-y-1">
          <label htmlFor={protoId} className="block font-semibold">Já tem um protocolo? (opcional)</label>
          <input
            id={protoId}
            type="text"
            autoComplete="off"
            maxLength={60}
            value={protocol}
            onChange={(e) => setProtocol(e.target.value)}
            aria-invalid={protocolInvalid}
            aria-describedby={protocolInvalid ? errId : undefined}
            className="min-h-12 w-full rounded-control border border-line bg-surface px-4 text-lg sm:max-w-xs"
          />
          {protocolInvalid && (
            <p id={errId} className="text-sm font-medium text-danger">
              Use só letras, números, hífen, ponto, barra e espaço (até 40).
            </p>
          )}
        </div>
        <Button disabled={n === 0 || protocolInvalid || mutation.isPending} onClick={() => mutation.mutate(key)}>
          {mutation.isPending ? 'Gerando…' : stale ? 'Gerar de novo' : 'Gerar contestação'}
        </Button>
        <div aria-live="polite">
          {mutation.isError && <p role="alert" className="font-medium text-danger">{mutation.error.message}</p>}
        </div>
      </Card>

      {done && (
        <Card className="space-y-5">
          {stale && (
            <div role="status" className="rounded-control border border-warning-line bg-warning-soft p-3 font-medium text-warning">
              Este texto está desatualizado. Você mudou a seleção ou os dados. Toque em "Gerar de novo".
            </div>
          )}
          <div className={stale ? 'space-y-5 opacity-60' : 'space-y-5'}>
            <div>
              <h3 className="text-xl font-bold">Seu texto pronto</h3>
              <p className="text-muted">Total contestado: <span className="font-semibold text-ink">{formatBRL(done.dispute.total_cents)}</span>. Cole no chat, e-mail ou formulário.</p>
            </div>
            <CopyText text={done.dispute.text} />
            <p className="rounded-control bg-accent-soft p-3 text-sm text-ink">{done.dispute.disclaimer}</p>
            <Roadmap dispute={done.dispute} />
          </div>
        </Card>
      )}
    </section>
  )
}
