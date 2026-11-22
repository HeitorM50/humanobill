import { useRef, useState, type ReactNode } from 'react'
import { useMutation } from '@tanstack/react-query'
import { createDispute } from '../../lib/api'
import { letterPath } from '../../lib/letter'
import { recordDispute } from '../../lib/history'
import { formatBRL } from '../../lib/format'
import type { Analysis } from '../../types/Analysis'
import type { DisputeCallStep } from '../../types/DisputeCallStep'
import type { DisputeResult } from '../../types/DisputeResult'
import { Alert, Button, ButtonLink, Card, Field, Icon } from '../ui'
import { DISPUTE_SECTION_ID } from './RecoverableCard'

// Mesma regra do servidor: só dígitos e hífen, até 30 caracteres.
export const PROTOCOL_RE = /^[0-9-]*$/
export const PROTOCOL_MAX = 30
export const PROTOCOL_ERROR = 'Use só números e hífen (até 30).'

type Mode = 'escrever' | 'ligar'

export function CopyText({
  text,
  label = 'Texto da contestação',
  buttonLabel = 'Copiar',
  children,
}: {
  text: string
  label?: string
  buttonLabel?: string
  /** Se vier, é o que aparece na tela; `text` continua sendo o que vai para a área de transferência. */
  children?: ReactNode
}) {
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
        aria-label={label}
        className={`max-w-text rounded-control border border-line bg-bg p-5 font-normal break-words text-ink ${
          children ? 'text-body' : 'text-h4 leading-relaxed whitespace-pre-wrap'
        }`}
      >
        {children ?? text}
      </div>
      <div className="flex flex-wrap items-center gap-3">
        <Button variant="tertiary" onClick={copy}>{state === 'copied' ? 'Copiado!' : buttonLabel}</Button>
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

/** Roteiro em texto puro, para colar em um bloco de notas ou imprimir. */
function scriptText(steps: DisputeCallStep[]) {
  return steps.map((p, i) => `${i + 1}. ${p.title}\n${p.say}`).join('\n\n')
}

export function CallScript({ steps, stale }: { steps: DisputeCallStep[]; stale: boolean }) {
  return (
    <div className={stale ? 'space-y-4 opacity-60' : 'space-y-4'}>
      <div className="max-w-text">
        <h3 className="text-h3">Roteiro para ligar</h3>
        <p className="text-muted">Ligue para o SAC e siga os passos na ordem. É só ler cada trecho.</p>
      </div>
      <CopyText text={scriptText(steps)} label="Roteiro da ligação (texto para copiar)" buttonLabel="Copiar roteiro">
        <ol aria-label="Roteiro da ligação" className="space-y-5">
          {steps.map((p, i) => (
            <li key={p.title} className="flex gap-3">
              <span aria-hidden="true" className="flex size-8 shrink-0 items-center justify-center rounded-full bg-accent font-bold text-on-accent">
                {i + 1}
              </span>
              <div className="min-w-0">
                <h4 className="text-h4 font-bold">{p.title}</h4>
                <p className="mt-1">{p.say}</p>
              </div>
            </li>
          ))}
        </ol>
      </CopyText>
    </div>
  )
}

export function Roadmap({ dispute }: { dispute: DisputeResult }) {
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
                  className="mt-1 inline-flex min-h-11.5 items-center gap-1.5 font-semibold text-accent underline underline-offset-4 hover:text-accent-strong"
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

/** Link para a carta formal (PDF). Abre em outra aba para não perder a seleção. */
export function CartaLink({ to }: { to: string }) {
  return (
    <ButtonLink to={to} variant="secondary" target="_blank" rel="noopener noreferrer">
      Carta formal (PDF)
      <span className="sr-only"> (abre em nova aba)</span>
    </ButtonLink>
  )
}

export function DisputeSection({ analysis, selectedIds }: { analysis: Analysis; selectedIds: string[] }) {
  const [mode, setMode] = useState<Mode>('escrever')
  const [protocol, setProtocol] = useState('')
  const [done, setDone] = useState<{ dispute: DisputeResult; key: string; baseKey: string; ids: string[]; protocol: string } | null>(null)

  const trimmed = protocol.trim()
  const protocolInvalid = trimmed.length > PROTOCOL_MAX || !PROTOCOL_RE.test(trimmed)
  // `baseKey` muda com a seleção ou o "pago"; `key` também muda com o protocolo.
  const baseKey = JSON.stringify([[...selectedIds].sort(), analysis.paid])
  const key = JSON.stringify([baseKey, trimmed])

  const mutation = useMutation({
    mutationFn: (k: { key: string; baseKey: string }) =>
      createDispute(analysis.id, { finding_ids: selectedIds, ...(trimmed ? { protocol: trimmed } : {}) }).then((dispute) => {
        // Com protocolo, a lista "Minhas contas" passa a contar o prazo da empresa.
        if (trimmed) recordDispute(analysis, trimmed)
        return { dispute, ...k, ids: selectedIds, protocol: trimmed }
      }),
    onSuccess: setDone,
  })
  const generate = () => mutation.mutate({ key, baseKey })

  const total = analysis.findings.filter((f) => selectedIds.includes(f.id)).reduce((t, f) => t + f.amount_cents, 0)
  const n = selectedIds.length
  const calling = mode === 'ligar'
  const staleBase = done != null && done.baseKey !== baseKey
  const stale = done != null && done.key !== key
  const callScript = done?.dispute.call_script ?? []

  const protocolField = (label: string, hint?: string) => (
    <Field label={label} hint={hint} error={protocolInvalid ? PROTOCOL_ERROR : undefined}>
      {(c) => (
        <input
          {...c}
          className={`${c.className} sm:max-w-xs`}
          type="text"
          inputMode="numeric"
          autoComplete="off"
          maxLength={60}
          value={protocol}
          onChange={(e) => setProtocol(e.target.value)}
        />
      )}
    </Field>
  )

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
        <div role="group" aria-label="Como você prefere contestar?" className="flex flex-wrap gap-3">
          <Button variant={calling ? 'secondary' : 'primary'} aria-pressed={!calling} onClick={() => setMode('escrever')} className="flex-1 basis-40">
            Prefiro escrever
          </Button>
          <Button variant={calling ? 'primary' : 'secondary'} aria-pressed={calling} onClick={() => setMode('ligar')} className="flex-1 basis-40">
            Prefiro ligar
          </Button>
        </div>
        {!calling && protocolField('Já tem um protocolo? (opcional)')}
        <Button disabled={n === 0 || protocolInvalid} loading={mutation.isPending} loadingLabel="Gerando…" onClick={generate}>
          {stale && !calling ? 'Gerar de novo' : calling ? (staleBase ? 'Gerar roteiro de novo' : 'Gerar roteiro') : 'Gerar contestação'}
        </Button>
        <div aria-live="polite">
          {mutation.isError && (
            <Alert tone="danger" role="alert">
              <p>{mutation.error.message}</p>
            </Alert>
          )}
        </div>
      </Card>

      {done && calling && (
        <Card className="space-y-6 md:p-6">
          {staleBase && (
            <Alert tone="warning" role="status">
              <p>Este roteiro está desatualizado. Você mudou a seleção ou os dados. Toque em "Gerar roteiro de novo".</p>
            </Alert>
          )}
          <CallScript steps={callScript} stale={staleBase} />
          <div className="max-w-text space-y-3">
            {protocolField('Anote o protocolo aqui', 'Depois da ligação, escreva o número que o atendente passar. A gente coloca no texto da contestação.')}
            <Button variant="secondary" disabled={trimmed === '' || protocolInvalid} loading={mutation.isPending} loadingLabel="Gerando…" onClick={generate}>
              Refazer contestação com o protocolo
            </Button>
          </div>
        </Card>
      )}

      {done && (
        <Card className="space-y-6 md:p-6">
          {stale && (
            <Alert tone="warning" role="status">
              <p>
                {calling && !staleBase
                  ? 'Este texto ainda não tem o protocolo que você anotou. Toque em "Refazer contestação com o protocolo".'
                  : calling
                    ? 'Este texto está desatualizado. Você mudou a seleção ou os dados. Toque em "Gerar roteiro de novo".'
                    : 'Este texto está desatualizado. Você mudou a seleção ou os dados. Toque em "Gerar de novo".'}
              </p>
            </Alert>
          )}
          <div className={stale ? 'space-y-6 opacity-60' : 'space-y-6'}>
            <div className="max-w-text">
              <h3 className="text-h3">Seu texto pronto</h3>
              <p className="text-muted">Total contestado: <span className="font-semibold text-ink">{formatBRL(done.dispute.total_cents)}</span>. Cole no chat, e-mail ou formulário.</p>
            </div>
            <CopyText text={done.dispute.text} />
            <div className="max-w-text space-y-2">
              <CartaLink to={letterPath(analysis.id, done.ids, done.protocol)} />
              <p className="text-caption text-muted">Abre a carta pronta para imprimir ou salvar em PDF.</p>
            </div>
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
