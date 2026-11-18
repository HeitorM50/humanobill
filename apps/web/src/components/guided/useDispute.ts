import { useState } from 'react'
import { useMutation } from '@tanstack/react-query'
import { createDispute } from '../../lib/api'
import type { Analysis } from '../../types/Analysis'
import type { DisputeResult } from '../../types/DisputeResult'
import type { Finding } from '../../types/Finding'

/** Os achados que podem ser contestados: tudo o que não está "ok". */
export const contestable = (a: Analysis): Finding[] => a.findings.filter((f) => f.severity !== 'ok')

/**
 * A contestação do passo a passo. Pede ao servidor o texto, o roteiro e os prazos de uma vez
 * (mesma chamada da versão completa) e guarda o resultado pela seleção: se a pessoa muda o que
 * contestar ou responde "já paguei", o resultado antigo deixa de valer e o texto é gerado de novo.
 */
export function useDispute(analysis: Analysis, selectedIds: string[]) {
  const key = JSON.stringify([[...selectedIds].sort(), analysis.paid])
  const [done, setDone] = useState<{ key: string; dispute: DisputeResult } | null>(null)
  // A chave já pedida: evita pedir de novo sozinho depois de um erro (quem decide é o botão "Tentar de novo").
  const [requested, setRequested] = useState<string | null>(null)

  const mutation = useMutation({
    mutationFn: (k: string) => createDispute(analysis.id, { finding_ids: selectedIds }).then((dispute) => ({ key: k, dispute })),
    onSuccess: setDone,
  })

  const request = () => {
    setRequested(key)
    mutation.mutate(key)
  }
  /** Chame ao abrir uma tela que precisa do texto: só pede se ainda não pediu para esta seleção. */
  const ensure = () => {
    if (requested !== key && done?.key !== key) request()
  }

  return {
    key,
    dispute: done?.key === key ? done.dispute : null,
    pending: mutation.isPending,
    error: mutation.isError && requested === key ? mutation.error : null,
    request,
    ensure,
  }
}
export type Dispute = ReturnType<typeof useDispute>
