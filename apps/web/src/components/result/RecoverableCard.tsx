import { useEffect, useRef, useState } from 'react'
import { animate, motion, useReducedMotion } from 'motion/react'
import { formatBRL } from '../../lib/format'
import type { Analysis } from '../../types/Analysis'

export const PAID_GROUP_ID = 'pergunta-pago'

/** Leva o foco até a pergunta "Você já pagou esta fatura?" (sem duplicá-la). */
function focusPaidQuestion() {
  const group = document.getElementById(PAID_GROUP_ID)
  group?.scrollIntoView?.({ block: 'center', behavior: 'smooth' })
  group?.querySelector<HTMLButtonElement>('button')?.focus({ preventScroll: true })
}

/** Número que "conta" até o valor final. Decorativo: o anúncio fica na região aria-live. */
function CountUp({ cents }: { cents: number }) {
  const reduce = useReducedMotion()
  const [animated, setShown] = useState(0)
  const from = useRef(0)
  const shown = reduce ? cents : animated

  useEffect(() => {
    if (reduce) return
    const controls = animate(from.current, cents, {
      duration: 0.9,
      ease: [0.16, 1, 0.3, 1],
      onUpdate: (v) => {
        from.current = v
        setShown(Math.round(v))
      },
    })
    return () => controls.stop()
  }, [cents, reduce])

  return <span aria-hidden="true">{formatBRL(shown)}</span>
}

const sum = (a: Analysis, sev: 'indevido' | 'suspeito') =>
  a.findings.filter((f) => f.severity === sev).reduce((t, f) => t + f.amount_cents, 0)

export function RecoverableCard({ analysis }: { analysis: Analysis }) {
  const hasUndue = analysis.findings.some((f) => f.severity === 'indevido')
  const suspect = sum(analysis, 'suspeito')
  const paid = analysis.paid

  if (!hasUndue) {
    if (!analysis.findings.some((f) => f.severity === 'suspeito')) return null
    return (
      <div aria-live="polite" className="rounded-card border border-warning-line bg-warning-soft px-5 py-4">
        <p className="font-semibold text-warning">
          {formatBRL(suspect)} para conferir <span className="font-normal text-ink">— responda as perguntas abaixo.</span>
        </p>
      </div>
    )
  }

  const undue = sum(analysis, 'indevido')
  const hero = paid === null ? undue : analysis.recoverable_cents
  const lead = paid === null ? null : 'Até'
  const tail =
    paid === null
      ? 'parecem indevidos'
      : paid
        ? 'recuperáveis (devolução em dobro, CDC art. 42)'
        : 'podem ser retirados da fatura'
  const spoken = `${lead ? lead + ' ' : ''}${formatBRL(hero)} ${tail}`

  return (
    <motion.section
      aria-label="Valor que parece indevido"
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.4 }}
      className="relative overflow-hidden rounded-card bg-linear-to-br from-accent to-accent-strong p-6 text-white shadow-lift"
    >
      <div aria-hidden="true" className="pointer-events-none absolute -right-10 -top-12 size-44 rounded-full bg-white/10" />
      <p className="sr-only" aria-live="polite" aria-atomic="true">{spoken}</p>
      <div aria-hidden="true" className="relative">
        <p className="text-sm font-semibold uppercase tracking-wide text-white/80">
          {paid === null ? 'Parece indevido' : 'Valor recuperável'}
        </p>
        <p className="mt-1 flex flex-wrap items-baseline gap-x-2">
          {lead && <span className="text-xl font-semibold">{lead}</span>}
          <span className="text-5xl font-extrabold leading-none tracking-tight tabular-nums">
            <CountUp cents={hero} />
          </span>
        </p>
        <p className="mt-2 text-lg font-semibold">{tail}</p>
      </div>
      {paid === true && (
        <p className="relative mt-3 text-sm text-white/90">Estimativa. O valor final depende da empresa ou do órgão</p>
      )}
      {paid === null && (
        <div className="relative mt-4 flex flex-wrap items-center gap-3">
          <span className="font-semibold">Já pagou? Pode ser o dobro.</span>
          <button
            type="button"
            onClick={focusPaidQuestion}
            className="inline-flex min-h-12 items-center rounded-control bg-white px-5 font-semibold text-accent-strong transition-colors hover:bg-accent-soft focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white"
          >
            Responder
          </button>
        </div>
      )}
      <p className="relative mt-4 text-xs text-white/80">Conteúdo informativo, não é orientação jurídica</p>
    </motion.section>
  )
}
