import { useState } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { patchAnalysis, type AnalysisPatch } from '../../lib/api'
import type { Analysis } from '../../types/Analysis'
import type { Answer } from '../../types/Answer'

/**
 * Respostas e "já pagou?" de uma análise. A decisão continua sendo do servidor: aqui só se
 * manda o PATCH e se guarda o resultado no cache. Usado pelo painel e pelo modo passo a passo.
 */
export function useAnalysisAnswers(analysis: Analysis) {
  const qc = useQueryClient()
  const [paidTouched, setPaidTouched] = useState(false)

  const mutation = useMutation({
    mutationFn: (body: AnalysisPatch) => patchAnalysis(analysis.id, body),
    onSuccess: (next) => qc.setQueryData(['analysis', analysis.id], next),
  })

  // PATCH substitui o mapa inteiro: sempre manda as respostas atuais + a nova.
  const answer = (qid: string, a: Answer) => mutation.mutate({ answers: { ...analysis.answers, [qid]: a } })
  const setPaid = (paid: boolean | null) => {
    setPaidTouched(true)
    mutation.mutate({ paid })
  }
  return { answer, setPaid, paidTouched, busy: mutation.isPending, error: mutation.isError ? mutation.error : null }
}
