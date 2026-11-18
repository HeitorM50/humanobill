import { useEffect, useEffectEvent, useMemo, useState } from 'react'
import type { Analysis } from '../../types/Analysis'
import type { Finding } from '../../types/Finding'
import { useAnalysisAnswers } from '../result/useAnalysisAnswers'
import { ChooseScreen, CallScreen, TextScreen, WhereScreen } from './ContestScreens'
import { NoQuestionsScreen, PaidScreen, QuestionScreen } from './ConfirmScreens'
import { CompareScreen, CompositionScreen, EnergyScreen, ReminderScreen } from './ExtraScreens'
import { GuidedShell } from './GuidedShell'
import { FindingScreen, SummaryScreen } from './ResultScreens'
import { SendChoice, SentSummary } from './SendScreen'
import { contestable, useDispute } from './useDispute'
import { SCREEN, STEP_NAMES, buildScreens, findingKey, firstScreenKey, questionKey, stepProgress } from './screens'

/**
 * O modo passo a passo (RF22): uma tela por vez, com "Voltar" e "Próximo". O estado (respostas,
 * escolha do que contestar, texto gerado) fica aqui, acima das telas, por isso voltar não perde nada.
 * Sem `analysis`, só existe a primeira tela (enviar a fatura).
 */
export function GuidedFlow({ analysis }: { analysis: Analysis | null }) {
  const screens = useMemo(() => buildScreens(analysis), [analysis])
  const [pos, setPos] = useState(() => {
    const first = firstScreenKey(screens, analysis !== null)
    return { key: first, index: Math.max(0, screens.findIndex((s) => s.key === first)) }
  })
  const [navigated, setNavigated] = useState(false)

  // A posição vale pela `key`. Se a tela atual some (ex.: uma resposta tirou o último problema), fica na mais próxima.
  const found = screens.findIndex((s) => s.key === pos.key)
  const index = found >= 0 ? found : Math.min(pos.index, screens.length - 1)
  const screen = screens[index]

  const go = (delta: number) => {
    const to = Math.min(Math.max(index + delta, 0), screens.length - 1)
    setNavigated(true)
    setPos({ key: screens[to].key, index: to })
  }

  // Respostas e "já pagou?" (mesmo PATCH da versão completa). Sem análise, o hook não é usado.
  const answers = useAnalysisAnswers(analysis ?? PLACEHOLDER)

  // Escolhas explícitas do que contestar. Sem escolha, vale o padrão: indevido marcado, suspeito não.
  const [overrides, setOverrides] = useState<Record<string, boolean>>({})
  const isChecked = (f: Finding) => overrides[f.id] ?? f.severity === 'indevido'
  const selectedIds = analysis ? contestable(analysis).filter(isChecked).map((f) => f.id) : []
  const dispute = useDispute(analysis ?? PLACEHOLDER, selectedIds)

  // Ao abrir uma tela que precisa do texto da contestação, pede (uma vez por seleção).
  const needsDispute = screen.key === SCREEN.texto || screen.key === SCREEN.onde || screen.key === SCREEN.ligar
  const hasSelection = selectedIds.length > 0
  const ensureDispute = useEffectEvent(() => dispute.ensure())
  useEffect(() => {
    if (needsDispute && hasSelection) ensureDispute()
  }, [needsDispute, hasSelection, dispute.key])

  const { n, total } = stepProgress(screens, screen, analysis !== null)
  const fullHref = analysis ? `/r/${analysis.id}` : '/analisar'
  const isFirst = index === 0
  const isLast = index === screens.length - 1

  let next: React.ComponentProps<typeof GuidedShell>['next'] = { label: 'Próximo', onClick: () => go(1) }
  if (!analysis) next = null
  else if (isLast) next = { label: 'Terminar', to: '/' }
  else if (screen.key === SCREEN.escolher && selectedIds.length === 0) next = { label: 'Próximo', onClick: () => go(1), disabled: true }
  else if (screen.key === SCREEN.texto && !dispute.dispute) next = { label: 'Próximo', onClick: () => go(1), disabled: true }
  const back = isFirst ? { to: '/' } : { onClick: () => go(-1) }

  return (
    <>
      {/* Anuncia cada tela nova. O foco também vai para o título, mas nem todo leitor de tela lê os dois. */}
      <p data-testid="anuncio" aria-live="polite" aria-atomic="true" className="sr-only">
        {`Passo ${n} de ${total}: ${screen.title}`}
      </p>
      <GuidedShell
        key={screen.key}
        step={n}
        total={total}
        stepName={STEP_NAMES[screen.step]}
        title={screen.title}
        moveFocus={navigated}
        back={back}
        next={next}
        fullHref={fullHref}
      >
        {renderScreen()}
      </GuidedShell>
    </>
  )

  function renderScreen() {
    if (!analysis) return <SendChoice />
    const k = screen.key
    if (k === SCREEN.enviar) return <SentSummary analysis={analysis} />
    if (k === SCREEN.pago) return <PaidScreen analysis={analysis} answers={answers} />
    if (k === SCREEN.semPerguntas) return <NoQuestionsScreen />
    if (k === SCREEN.resumo) return <SummaryScreen analysis={analysis} />
    if (k === SCREEN.composicao) return <CompositionScreen analysis={analysis} />
    if (k === SCREEN.energia) return <EnergyScreen analysis={analysis} />
    if (k === SCREEN.lembrete) return <ReminderScreen analysis={analysis} />
    if (k === SCREEN.comparar) return <CompareScreen analysis={analysis} />
    if (k === SCREEN.escolher)
      return <ChooseScreen analysis={analysis} isChecked={isChecked} onChange={(id, v) => setOverrides((o) => ({ ...o, [id]: v }))} />
    if (k === SCREEN.texto) return <TextScreen dispute={dispute} />
    if (k === SCREEN.onde) return <WhereScreen dispute={dispute} />
    if (k === SCREEN.ligar) return <CallScreen dispute={dispute} />

    const question = analysis.questions.find((q) => questionKey(q.id) === k)
    if (question) return <QuestionScreen analysis={analysis} question={question} answers={answers} />
    const finding = analysis.findings.find((f) => findingKey(f.id) === k)
    if (finding) return <FindingScreen finding={finding} items={analysis.bill.items.filter((i) => finding.item_refs.includes(i.id))} />
    return null
  }
}

/**
 * Os hooks de resposta e de contestação precisam de uma análise mesmo na primeira tela (sem fatura ainda).
 * Nunca chegam a ser usados nesse caso: não há pergunta, nem achado, nem botão que os chame.
 */
const PLACEHOLDER = { id: '', answers: {}, paid: null, findings: [], questions: [] } as unknown as Analysis
