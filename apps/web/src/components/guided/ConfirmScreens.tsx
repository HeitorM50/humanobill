import { useId } from 'react'
import type { Analysis } from '../../types/Analysis'
import type { Question } from '../../types/Question'
import { AmountField, Choice, DateField, YesNo } from '../result/QuestionsPanel'
import type { useAnalysisAnswers } from '../result/useAnalysisAnswers'
import { Alert } from '../ui'

type Answers = ReturnType<typeof useAnalysisAnswers>

/** "Salvando…" e erro de uma resposta, anunciados com educação. */
function SaveStatus({ answers }: { answers: Answers }) {
  return (
    <div aria-live="polite" className="min-h-6">
      {answers.busy && <p className="font-semibold text-muted">Salvando…</p>}
      {answers.error && (
        <Alert tone="danger" role="alert">
          <p>{answers.error.message}</p>
        </Alert>
      )}
    </div>
  )
}

/** Uma pergunta de confirmação (RF09) sozinha na tela. As mesmas peças do painel da versão completa. */
export function QuestionScreen({ analysis, question, answers }: { analysis: Analysis; question: Question; answers: Answers }) {
  const cur = analysis.answers[question.id]
  const { busy, answer } = answers
  return (
    <>
      <p className="max-w-text text-muted">Se não souber, pode seguir sem responder. Sua resposta ajusta o resultado na hora.</p>
      {question.kind === 'yes_no' && <YesNo large question={question} current={cur} busy={busy} onAnswer={(a) => answer(question.id, a)} />}
      {question.kind === 'amount' && (
        <AmountField
          large
          question={question}
          current={cur?.type === 'amount' ? cur.value : undefined}
          busy={busy}
          onSubmit={(value) => answer(question.id, { type: 'amount', value })}
        />
      )}
      {question.kind === 'date' && (
        <DateField
          large
          question={question}
          current={cur?.type === 'date' ? cur.value : undefined}
          busy={busy}
          onSubmit={(value) => answer(question.id, { type: 'date', value })}
        />
      )}
      <SaveStatus answers={answers} />
    </>
  )
}

/** "Você já pagou esta fatura?" sozinha na tela: paguei, não paguei ou não sei. */
export function PaidScreen({ analysis, answers }: { analysis: Analysis; answers: Answers }) {
  const gid = useId()
  const { busy, setPaid, paidTouched } = answers
  return (
    <>
      <p id={gid} className="max-w-text text-muted">Se você já pagou, pode ter direito a receber de volta. Isso muda o valor que aparece no resultado.</p>
      <div role="group" aria-labelledby={gid} className="grid gap-3">
        <Choice large pressed={analysis.paid === true} disabled={busy} onClick={() => setPaid(true)}>Já paguei</Choice>
        <Choice large pressed={analysis.paid === false} disabled={busy} onClick={() => setPaid(false)}>Ainda não paguei</Choice>
        <Choice large pressed={analysis.paid === null && paidTouched} disabled={busy} onClick={() => setPaid(null)}>Não sei</Choice>
      </div>
      <SaveStatus answers={answers} />
    </>
  )
}

/** Etapa 2 sem nada para perguntar. */
export function NoQuestionsScreen() {
  return (
    <Alert tone="ok">
      <p className="text-h4">Não preciso perguntar nada sobre esta fatura. Pode seguir.</p>
    </Alert>
  )
}
