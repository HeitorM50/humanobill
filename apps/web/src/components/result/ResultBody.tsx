import { useCallback, useState, type ReactNode } from 'react'
import { AnimatePresence, motion } from 'motion/react'
import allClearUrl from '../../assets/tudo-certo.svg'
import type { Analysis } from '../../types/Analysis'
import type { Finding } from '../../types/Finding'
import { Alert } from '../ui'
import { BillItems } from './BillItems'
import { Composition } from './Composition'
import { EnergyPanel } from './EnergyPanel'
import { FindingCard } from './FindingCard'
import { ListenButton } from './ListenButton'
import { CorrectionDrawer } from './CorrectionDrawer'
import { DisputeSection } from './DisputeSection'
import { QuestionsPanel } from './QuestionsPanel'

function AllClear({ pendingQuestions }: { pendingQuestions: number }) {
  return (
    <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}>
      <Alert tone="ok" title="Tudo certo com esta conta">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:gap-4">
          <img src={allClearUrl} alt="" aria-hidden="true" width={84} height={84} className="size-21 shrink-0" />
          <div className="space-y-2">
            <p>Não achei nada fora do lugar nos itens abaixo.</p>
            {pendingQuestions > 0 && (
              <p className="font-semibold">
                Ainda tem {pendingQuestions === 1 ? 'uma pergunta' : `${pendingQuestions} perguntas`} sem resposta. Responder pode mudar este resultado.
              </p>
            )}
          </div>
        </div>
      </Alert>
    </motion.div>
  )
}

/**
 * Tudo o que fica abaixo do bloco do valor, dentro da largura máxima da página.
 * `children` entra no fim (aviso de exemplo, rodapé), na mesma coluna.
 */
export function ResultBody({ analysis, children }: { analysis: Analysis; children?: ReactNode }) {
  const { bill, findings, questions, answers } = analysis
  const hasProblems = findings.some((f) => f.severity !== 'ok')
  const pending = questions.filter((q) => !(q.id in answers)).length

  // Escolhas explícitas por id. Sem escolha, vale o padrão: indevido marcado, suspeito não.
  const [overrides, setOverrides] = useState<Record<string, boolean>>({})
  const isChecked = (f: Finding) => overrides[f.id] ?? f.severity === 'indevido'
  const setChecked = useCallback((id: string, v: boolean) => setOverrides((o) => ({ ...o, [id]: v })), [])
  const selectedIds = findings.filter((f) => f.severity !== 'ok' && isChecked(f)).map((f) => f.id)

  return (
    <div className="mx-auto w-full max-w-page space-y-8 px-5 py-8 md:space-y-14 md:px-10 md:py-14">
      <QuestionsPanel analysis={analysis} />

      {/* Achados à esquerda, a fatura à direita. No celular, os achados vêm primeiro. */}
      <div className="grid gap-8 md:grid-cols-2 md:items-start md:gap-6">
        <section aria-labelledby="prob-t" className="min-w-0 space-y-4">
          <h2 id="prob-t" className="text-h2">Problemas encontrados</h2>
          <ListenButton analysis={analysis} />
          <AnimatePresence initial={false}>{!hasProblems && <AllClear key="clear" pendingQuestions={pending} />}</AnimatePresence>
          <div className="space-y-4">
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

        <section aria-labelledby="itens-t" className="min-w-0 space-y-4">
          <h2 id="itens-t" className="text-h2">Sua conta explicada</h2>
          <BillItems bill={bill} findings={findings} />
          <CorrectionDrawer analysis={analysis} />
        </section>
      </div>

      {hasProblems && <DisputeSection analysis={analysis} selectedIds={selectedIds} />}

      <Composition bill={bill} />

      {bill.energy && <EnergyPanel energy={bill.energy} referenceMonth={bill.reference_month} />}

      {children}
    </div>
  )
}
