import { useMutation } from '@tanstack/react-query'
import { useNavigate, useSearchParams } from 'react-router'
import { DEMO_SAMPLES, uploadSample } from '../../lib/api'
import type { Analysis } from '../../types/Analysis'
import { Alert, Button, Card } from '../ui'

/** Só em resultados de exemplo: reenvia o PDF do exemplo pela extração real. */
export function LiveDemo({ analysis }: { analysis: Analysis }) {
  const navigate = useNavigate()
  const [params] = useSearchParams()
  const wanted = params.get('exemplo')
  const sample =
    DEMO_SAMPLES.find((s) => s.id === wanted) ??
    DEMO_SAMPLES.find((s) => s.kind === (analysis.bill.kind === 'energy' ? 'Energia' : 'Telecom'))
  const live = useMutation({ mutationFn: uploadSample, onSuccess: (a) => navigate(`/r/${a.id}`) })

  if (analysis.source !== 'demo' || !sample) return null

  return (
    <Card className="space-y-3">
      <h2 className="text-h3">Quer ver a IA lendo o PDF?</h2>
      <p className="max-w-text text-muted">Vamos enviar esta fatura de exemplo (sintética, sem dados reais) para a IA ler ao vivo. Pode levar alguns segundos.</p>
      <Button variant="secondary" loading={live.isPending} loadingLabel="Lendo a fatura…" onClick={() => live.mutate(sample.id)}>
        Analisar com IA de verdade
      </Button>
      {live.isError && (
        <Alert tone="danger" role="alert" className="max-w-text">
          <p>{live.error.message.replace(/[.\s]+$/, '')}. Você pode continuar com o exemplo pronto que está aqui.</p>
        </Alert>
      )}
    </Card>
  )
}
