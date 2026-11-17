import { useEffect, useId, useRef, useState, type ReactNode } from 'react'
import { animate, useReducedMotion } from 'motion/react'
import { formatBRL, formatMonth } from '../../lib/format'
import type { Analysis } from '../../types/Analysis'
import { Button } from '../ui'

export const PAID_GROUP_ID = 'pergunta-pago'
export const DISPUTE_SECTION_ID = 'contestar'
export const QUESTIONS_ID = 'perguntas-rapidas'

/** Rola até a seção "Contestar" e leva o foco ao título. */
function focusDispute() {
  const el = document.getElementById(DISPUTE_SECTION_ID)
  el?.scrollIntoView?.({ block: 'start', behavior: 'smooth' })
  el?.querySelector<HTMLElement>('h2')?.focus({ preventScroll: true })
}

/** Leva o foco até a pergunta "Você já pagou esta fatura?" (sem duplicá-la). */
function focusPaidQuestion() {
  const group = document.getElementById(PAID_GROUP_ID)
  group?.scrollIntoView?.({ block: 'center', behavior: 'smooth' })
  group?.querySelector<HTMLButtonElement>('button')?.focus({ preventScroll: true })
}

/** Leva o foco ao primeiro campo do painel de perguntas rápidas. */
function focusFirstQuestion() {
  const panel = document.getElementById(QUESTIONS_ID)
  panel?.scrollIntoView?.({ block: 'center', behavior: 'smooth' })
  panel?.querySelector<HTMLElement>('button:not([disabled]), input:not([disabled])')?.focus({ preventScroll: true })
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

/** Bloco de cor de ponta a ponta; o conteúdo respeita a largura máxima da página. */
const INNER = 'mx-auto max-w-page px-5 py-8 md:px-10 md:py-14'

type Bill = Analysis['bill']

/** Estrutura comum do bloco azul: operadora/mês, rótulo, valor, frase de apoio e ações. */
function ValueBlock({
  ariaLabel,
  bill,
  label,
  lead,
  cents,
  tail,
  spoken,
  support,
  children,
}: {
  ariaLabel: string
  bill: Bill
  label: string
  lead?: string | null
  cents: number
  tail?: string
  spoken: string
  support: string
  children: ReactNode
}) {
  return (
    <section aria-label={ariaLabel} className="bg-accent text-on-accent">
      <div className={INNER}>
        <p className="text-caption">
          {bill.provider_name} · {formatMonth(bill.reference_month)}
        </p>
        <p className="sr-only" aria-live="polite" aria-atomic="true">{spoken}</p>
        <div aria-hidden="true" className="mt-3 space-y-3">
          <p className="text-h4 font-semibold">{label}</p>
          <p className="flex flex-wrap items-baseline gap-x-3 font-display text-hero font-bold tracking-display">
            {lead && <span className="text-h3">{lead}</span>}
            <span>
              <span className="marca-texto">
                <CountUp cents={cents} />
              </span>
            </span>
          </p>
          {tail && <p className="text-h4 font-semibold">{tail}</p>}
          <p>{support}</p>
        </div>
        {children}
      </div>
    </section>
  )
}

/** O valor do resultado (guia 9.3): bloco `accent` logo abaixo da navbar. */
export function RecoverableCard({ analysis }: { analysis: Analysis }) {
  const undueFindings = analysis.findings.filter((f) => f.severity === 'indevido')
  const paid = analysis.paid
  const paidHintId = useId()
  const { bill } = analysis

  if (undueFindings.length === 0) {
    const suspectFindings = analysis.findings.filter((f) => f.severity === 'suspeito')
    if (suspectFindings.length === 0) return null
    const suspect = sum(analysis, 'suspeito')
    const n = suspectFindings.length
    return (
      <ValueBlock
        ariaLabel="Valor para conferir"
        bill={bill}
        label="Para conferir"
        cents={suspect}
        spoken={`${formatBRL(suspect)} para conferir`}
        support={`em ${n} ${n === 1 ? 'cobrança' : 'cobranças'} desta fatura de ${formatBRL(bill.total_cents)} — responda as perguntas abaixo`}
      >
        <div className="mt-6 flex flex-col gap-3 sm:flex-row sm:flex-wrap">
          <Button variant="tertiary" className="foco-sobre-azul" onClick={focusFirstQuestion}>Responder perguntas</Button>
        </div>
        <p className="mt-6 text-caption">Conteúdo informativo, não é orientação jurídica</p>
      </ValueBlock>
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
  const count = undueFindings.length

  return (
    <ValueBlock
      ariaLabel="Valor que parece indevido"
      bill={bill}
      label={paid === null ? 'Parece indevido' : 'Valor recuperável'}
      lead={lead}
      cents={hero}
      tail={tail}
      spoken={spoken}
      support={`em ${count} ${count === 1 ? 'cobrança' : 'cobranças'} desta fatura de ${formatBRL(bill.total_cents)}`}
    >
      {paid === true && <p className="mt-3 text-caption">Estimativa. O valor final depende da empresa ou do órgão</p>}
      {paid === null && (
        <p id={paidHintId} className="mt-6 font-semibold">Já pagou? Pode ser o dobro.</p>
      )}
      <div className={`${paid === null ? 'mt-3' : 'mt-6'} flex flex-col gap-3 sm:flex-row sm:flex-wrap`}>
        <Button variant="tertiary" className="foco-sobre-azul" onClick={focusDispute}>Contestar agora</Button>
        {paid === null && (
          <Button variant="onDark" aria-describedby={paidHintId} onClick={focusPaidQuestion}>Responder</Button>
        )}
      </div>
      <p className="mt-6 text-caption">Conteúdo informativo, não é orientação jurídica</p>
    </ValueBlock>
  )
}
