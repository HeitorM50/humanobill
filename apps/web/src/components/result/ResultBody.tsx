import { useCallback, useState } from 'react'
import { AnimatePresence, motion } from 'motion/react'
import type { Analysis } from '../../types/Analysis'
import type { Finding } from '../../types/Finding'
import { BillItems } from './BillItems'
import { EnergyPanel } from './EnergyPanel'
import { FindingCard } from './FindingCard'
import { DisputeSection } from './DisputeSection'
import { QuestionsPanel } from './QuestionsPanel'

function AllClear({ pendingQuestions }: { pendingQuestions: number }) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 8, scale: 0.98 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      exit={{ opacity: 0, scale: 0.98 }}
      className="rounded-card border border-ok-line bg-ok-soft p-6 text-center shadow-card"
    >
      <p className="text-4xl" aria-hidden="true">🎉</p>
      <p className="mt-2 text-2xl font-bold text-ok">Tudo certo com esta conta</p>
      <p className="mt-1 text-ink">Não achei nada fora do lugar nos itens abaixo.</p>
      {pendingQuestions > 0 && (
        <p className="mt-3 text-sm font-medium text-ink">
          Ainda tem {pendingQuestions === 1 ? 'uma pergunta' : `${pendingQuestions} perguntas`} sem resposta. Responder pode mudar este resultado.
        </p>
      )}
    </motion.div>
  )
}

export function ResultBody({ analysis }: { analysis: Analysis }) {
  const { bill, findings, questions, answers } = analysis
  const hasProblems = findings.some((f) => f.severity !== 'ok')
  const pending = questions.filter((q) => !(q.id in answers)).length

  // Escolhas explícitas por id. Sem escolha, vale o padrão: indevido marcado, suspeito não.
  const [overrides, setOverrides] = useState<Record<string, boolean>>({})
  const isChecked = (f: Finding) => overrides[f.id] ?? f.severity === 'indevido'
  const setChecked = useCallback((id: string, v: boolean) => setOverrides((o) => ({ ...o, [id]: v })), [])
  const selectedIds = findings.filter((f) => f.severity !== 'ok' && isChecked(f)).map((f) => f.id)

  return (
    <>
      <QuestionsPanel analysis={analysis} />

      <section aria-labelledby="prob-t" className="space-y-3">
        <h2 id="prob-t" className="text-2xl font-bold">Problemas encontrados</h2>
        <AnimatePresence initial={false}>{!hasProblems && <AllClear key="clear" pendingQuestions={pending} />}</AnimatePresence>
        <div className="space-y-3">
          {findings.map((f) => (
            <FindingCard
              key={f.id}
              finding={f}
              selection={f.severity === 'ok' ? undefined : { checked: isChecked(f), onChange: (v) => setChecked(f.id, v) }}
              items={bill.items.filter((i) => f.item_refs.includes(i.id))}
            />
          ))}
        </div>
      </section>

      {hasProblems && <DisputeSection analysis={analysis} selectedIds={selectedIds} />}

      {bill.energy && <EnergyPanel energy={bill.energy} referenceMonth={bill.reference_month} />}

      <section aria-labelledby="itens-t" className="space-y-3">
        <h2 id="itens-t" className="text-2xl font-bold">Sua conta explicada</h2>
        <BillItems items={bill.items} findings={findings} />
      </section>
    </>
  )
}
