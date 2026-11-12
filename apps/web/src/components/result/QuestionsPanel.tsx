import { useId, useState, type ReactNode } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { patchAnalysis, type AnalysisPatch } from '../../lib/api'
import { formatBRL } from '../../lib/format'
import type { Analysis } from '../../types/Analysis'
import type { Answer } from '../../types/Answer'
import type { Question } from '../../types/Question'
import { Button, Card } from '../ui'
import { PAID_GROUP_ID } from './RecoverableCard'

const choice =
  'inline-flex min-h-12 flex-1 items-center justify-center gap-2 rounded-control border px-4 py-2 text-base font-semibold transition-colors disabled:cursor-not-allowed disabled:opacity-60'
const choiceOff = 'border-line bg-surface text-ink hover:bg-accent-soft'
const choiceOn = 'border-accent bg-accent text-white'

function Choice({ pressed, disabled, onClick, children }: { pressed: boolean; disabled: boolean; onClick: () => void; children: ReactNode }) {
  return (
    <button type="button" aria-pressed={pressed} disabled={disabled} onClick={onClick} className={`${choice} ${pressed ? choiceOn : choiceOff}`}>
      {pressed && (
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <path d="M5 12.5l4.5 4.5L19 7.5" />
        </svg>
      )}
      {children}
    </button>
  )
}

function AmountField({ question, current, busy, onSubmit }: { question: Question; current: number | undefined; busy: boolean; onSubmit: (cents: number) => void }) {
  const inputId = useId()
  const [cents, setCents] = useState<number | null>(current ?? null)
  const saved = cents != null && cents === current
  return (
    <div className="space-y-2">
      <label htmlFor={inputId} className="block font-semibold">{question.text}</label>
      <div className="flex flex-col gap-2 sm:flex-row">
        <input
          id={inputId}
          type="text"
          inputMode="numeric"
          autoComplete="off"
          placeholder="R$ 0,00"
          value={cents == null ? '' : formatBRL(cents)}
          disabled={busy}
          onChange={(e) => {
            const digits = e.target.value.replace(/\D/g, '').slice(0, 9)
            setCents(digits ? Number(digits) : null)
          }}
          className="min-h-12 w-full rounded-control border border-line bg-surface px-4 text-lg tabular-nums disabled:opacity-60 sm:max-w-48"
        />
        <Button disabled={busy || cents == null || cents <= 0 || saved} onClick={() => cents != null && onSubmit(cents)}>
          {saved ? 'Salvo' : 'Confirmar'}
        </Button>
      </div>
    </div>
  )
}

// Rótulos por regra; qualquer regra fora do mapa usa Sim/Não.
const YES_NO_LABELS: Record<string, { yes: string; no: string }> = {
  'TEL-01': { yes: 'Contratei', no: 'Não contratei' },
  'ENE-07': { yes: 'Autorizei', no: 'Não autorizei' },
}
const DEFAULT_YES_NO = { yes: 'Sim', no: 'Não' }

function YesNo({ question, current, busy, onAnswer }: { question: Question; current: Answer | undefined; busy: boolean; onAnswer: (a: Answer) => void }) {
  const gid = useId()
  const labels = YES_NO_LABELS[question.rule_id] ?? DEFAULT_YES_NO
  return (
    <div role="group" aria-labelledby={gid} className="space-y-2">
      <p id={gid} className="font-semibold">{question.text}</p>
      <div className="flex flex-col gap-2 sm:flex-row">
        <Choice pressed={current?.type === 'yes'} disabled={busy} onClick={() => onAnswer({ type: 'yes' })}>{labels.yes}</Choice>
        <Choice pressed={current?.type === 'no'} disabled={busy} onClick={() => onAnswer({ type: 'no' })}>{labels.no}</Choice>
      </div>
    </div>
  )
}

function DateField({ question, current, busy, onSubmit }: { question: Question; current: string | undefined; busy: boolean; onSubmit: (iso: string) => void }) {
  const inputId = useId()
  return (
    <div className="space-y-2">
      <label htmlFor={inputId} className="block font-semibold">{question.text}</label>
      <input
        id={inputId}
        type="date"
        defaultValue={current ?? ''}
        disabled={busy}
        onChange={(e) => e.target.value && onSubmit(e.target.value)}
        className="min-h-12 w-full rounded-control border border-line bg-surface px-4 text-lg disabled:opacity-60 sm:max-w-56"
      />
    </div>
  )
}

export function QuestionsPanel({ analysis }: { analysis: Analysis }) {
  const qc = useQueryClient()
  const [paidTouched, setPaidTouched] = useState(false)
  const paidGroup = useId()

  const mutation = useMutation({
    mutationFn: (body: AnalysisPatch) => patchAnalysis(analysis.id, body),
    onSuccess: (next) => qc.setQueryData(['analysis', analysis.id], next),
  })
  const busy = mutation.isPending

  // PATCH substitui o mapa inteiro: sempre manda as respostas atuais + a nova.
  const answer = (qid: string, a: Answer) => mutation.mutate({ answers: { ...analysis.answers, [qid]: a } })
  const setPaid = (paid: boolean | null) => {
    setPaidTouched(true)
    mutation.mutate({ paid })
  }

  return (
    <section aria-labelledby="perg-t" className="space-y-3">
      <div>
        <h2 id="perg-t" className="text-2xl font-bold">Perguntas rápidas</h2>
        <p className="text-muted">Suas respostas ajustam o resultado na hora.</p>
      </div>
      <Card className="space-y-6">
        {analysis.questions.map((q) => {
          const cur = analysis.answers[q.id]
          if (q.kind === 'yes_no') return <YesNo key={q.id} question={q} current={cur} busy={busy} onAnswer={(a) => answer(q.id, a)} />
          if (q.kind === 'amount')
            return <AmountField key={q.id} question={q} current={cur?.type === 'amount' ? cur.value : undefined} busy={busy} onSubmit={(value) => answer(q.id, { type: 'amount', value })} />
          return <DateField key={q.id} question={q} current={cur?.type === 'date' ? cur.value : undefined} busy={busy} onSubmit={(value) => answer(q.id, { type: 'date', value })} />
        })}

        <div id={PAID_GROUP_ID} role="group" aria-labelledby={paidGroup} className="space-y-2">
          <p id={paidGroup} className="font-semibold">Você já pagou esta fatura?</p>
          <div className="flex flex-col gap-2 sm:flex-row">
            <Choice pressed={analysis.paid === true} disabled={busy} onClick={() => setPaid(true)}>Sim</Choice>
            <Choice pressed={analysis.paid === false} disabled={busy} onClick={() => setPaid(false)}>Não</Choice>
            <Choice pressed={analysis.paid === null && paidTouched} disabled={busy} onClick={() => setPaid(null)}>Não sei</Choice>
          </div>
        </div>

        <div aria-live="polite" className="min-h-6 text-sm">
          {busy && <p className="font-medium text-muted">Salvando…</p>}
          {mutation.isError && (
            <p role="alert" className="font-medium text-danger">
              {mutation.error.message}
            </p>
          )}
        </div>
      </Card>
    </section>
  )
}
