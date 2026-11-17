import { useRef, useState } from 'react'
import { useMutation } from '@tanstack/react-query'
import { createDispute } from '../../lib/api'
import { formatBRL } from '../../lib/format'
import type { Analysis } from '../../types/Analysis'
import type { Dispute } from '../../types/Dispute'
import { Alert, Button, Card, Field, Icon } from '../ui'
import { DISPUTE_SECTION_ID } from './RecoverableCard'

const PROTOCOL_RE = /^[\p{L}\p{N}\-./ ]*$/u

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
    <div className="space-y-4">
      {/* Citação grande: o texto é para ser lido e colado, então usa a fonte do texto, não a monoespaçada. */}
      <div
        ref={ref}
        tabIndex={0}
        role="region"
        aria-label="Texto da contestação"
        className="max-w-text rounded-control border border-line bg-bg p-5 text-h4 leading-relaxed font-normal whitespace-pre-wrap break-words text-ink"
      >
        {text}
      </div>
      <div className="flex flex-wrap items-center gap-3">
        <Button variant="tertiary" onClick={copy}>{state === 'copied' ? 'Copiado!' : 'Copiar'}</Button>
        <p role="status" aria-live="polite" className="font-semibold">
          {state === 'copied' && (
            <span className="inline-flex items-center gap-2 text-ok">
              <Icon name="ok" size={16} />
              Agora é só colar
            </span>
          )}
          {state === 'manual' && <span className="text-warning">Selecione e copie</span>}
        </p>
      </div>
    </div>
  )
}

function Roadmap({ dispute }: { dispute: Dispute }) {
  return (
    <section aria-labelledby="onde-t" className="max-w-text space-y-4">
      <h3 id="onde-t" className="text-h3">Onde reclamar</h3>
      {dispute.deadlines.length > 0 && (
        <Alert tone="warning" title="Fique de olho nos prazos">
          <ul className="space-y-2">
            {dispute.deadlines.map((d) => (
              <li key={d}>{d}</li>
            ))}
          </ul>
        </Alert>
      )}
      <ol className="space-y-3">
        {dispute.steps.map((s, i) => (
          <li key={s.title} className="flex gap-3 rounded-card border border-line bg-surface p-4">
            <span aria-hidden="true" className="flex size-8 shrink-0 items-center justify-center rounded-full bg-accent font-bold text-on-accent">
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
                  className="mt-1 inline-flex min-h-11 items-center gap-1.5 font-semibold text-accent underline underline-offset-4 hover:text-accent-strong"
                >
                  Abrir site
                  <Icon name="external" size={16} />
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
    <section id={DISPUTE_SECTION_ID} aria-labelledby="contestar-t" className="scroll-mt-4 space-y-4">
      <div className="max-w-text">
        <h2 id="contestar-t" tabIndex={-1} className="text-h2 outline-none">Contestar</h2>
        <p className="text-muted">Escolha os problemas acima e a gente escreve o texto pronto.</p>
      </div>
      <Card className="max-w-text space-y-4">
        <p className="text-h4 font-bold" aria-live="polite">
          {n === 0
            ? 'Nenhum problema selecionado'
            : `${n} ${n === 1 ? 'problema selecionado' : 'problemas selecionados'} · ${formatBRL(total)}`}
        </p>
        <Field
          label="Já tem um protocolo? (opcional)"
          error={protocolInvalid ? 'Use só letras, números, hífen, ponto, barra e espaço (até 40).' : undefined}
        >
          {(c) => (
            <input
              {...c}
              className={`${c.className} sm:max-w-xs`}
              type="text"
              autoComplete="off"
              maxLength={60}
              value={protocol}
              onChange={(e) => setProtocol(e.target.value)}
            />
          )}
        </Field>
        <Button disabled={n === 0 || protocolInvalid} loading={mutation.isPending} loadingLabel="Gerando…" onClick={() => mutation.mutate(key)}>
          {stale ? 'Gerar de novo' : 'Gerar contestação'}
        </Button>
        <div aria-live="polite">
          {mutation.isError && (
            <Alert tone="danger" role="alert">
              <p>{mutation.error.message}</p>
            </Alert>
          )}
        </div>
      </Card>

      {done && (
        <Card className="space-y-6 md:p-6">
          {stale && (
            <Alert tone="warning" role="status">
              <p>Este texto está desatualizado. Você mudou a seleção ou os dados. Toque em "Gerar de novo".</p>
            </Alert>
          )}
          <div className={stale ? 'space-y-6 opacity-60' : 'space-y-6'}>
            <div className="max-w-text">
              <h3 className="text-h3">Seu texto pronto</h3>
              <p className="text-muted">Total contestado: <span className="font-semibold text-ink">{formatBRL(done.dispute.total_cents)}</span>. Cole no chat, e-mail ou formulário.</p>
            </div>
            <CopyText text={done.dispute.text} />
            <Alert tone="info" className="max-w-text">
              <p>{done.dispute.disclaimer}</p>
            </Alert>
            <Roadmap dispute={done.dispute} />
          </div>
        </Card>
      )}
    </section>
  )
}
