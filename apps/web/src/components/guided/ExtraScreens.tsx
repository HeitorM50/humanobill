import { ButtonLink } from '../ui'
import type { Analysis } from '../../types/Analysis'
import { Composition } from '../result/Composition'
import { EnergyPanel } from '../result/EnergyPanel'
import { ReminderForm } from '../result/ReminderForm'
import type { ReminderDraft } from '../result/reminderDraft'

/** Etapa 4: os extras. Nenhum é obrigatório: o "Próximo" da moldura pula cada um. */
const SKIP_HINT = 'Isto é um extra: pode seguir sem fazer nada.'

export function CompositionScreen({ analysis }: { analysis: Analysis }) {
  return (
    <>
      <p className="max-w-text text-muted">Veja quanto de cada coisa você paga nesta fatura. {SKIP_HINT}</p>
      <Composition bill={analysis.bill} />
    </>
  )
}

export function EnergyScreen({ analysis }: { analysis: Analysis }) {
  if (!analysis.bill.energy) return null
  return (
    <>
      <p className="max-w-text text-muted">Veja se o seu gasto de luz subiu ou desceu. {SKIP_HINT}</p>
      <EnergyPanel energy={analysis.bill.energy} referenceMonth={analysis.bill.reference_month} />
    </>
  )
}

export function ReminderScreen({
  analysis,
  draft,
  onDraftChange,
}: {
  analysis: Analysis
  draft: ReminderDraft
  onDraftChange: (draft: ReminderDraft) => void
}) {
  return (
    <>
      <p className="max-w-text text-muted">{SKIP_HINT}</p>
      <ReminderForm analysisId={analysis.id} showHeading={false} draft={draft} onDraftChange={onDraftChange} />
    </>
  )
}

/** RF16: só um atalho para a tela de comparar. */
export function CompareScreen({ analysis }: { analysis: Analysis }) {
  return (
    <>
      <p className="max-w-text text-h4">Se você já analisou outra fatura, dá para ver o que mudou de um mês para o outro.</p>
      <p className="max-w-text text-muted">Essa tela abre a versão completa, com esta fatura já preenchida. {SKIP_HINT}</p>
      <ButtonLink to={`/comparar?a=${encodeURIComponent(analysis.id)}`} variant="secondary" className="min-h-14! w-full text-h4 sm:w-auto">
        Comparar duas faturas
      </ButtonLink>
    </>
  )
}
