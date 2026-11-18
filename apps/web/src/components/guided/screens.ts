import { composition } from '../../lib/derive'
import type { Analysis } from '../../types/Analysis'

/** As cinco etapas do modo passo a passo (RF22), na ordem. */
export type StepId = 'enviar' | 'confirmar' | 'resultado' | 'conhecer' | 'contestar'

export const STEP_NAMES: Record<StepId, string> = {
  enviar: 'Enviar',
  confirmar: 'Confirmar',
  resultado: 'Resultado',
  conhecer: 'Conhecer a conta',
  contestar: 'Contestar',
}

/** Uma tela do fluxo. A `key` é estável: a posição é guardada por ela, não pelo índice. */
export type Screen = { key: string; step: StepId; title: string }

/** Sem análise ainda, o total de passos é o do caminho completo. */
const DEFAULT_TOTAL = 5

export const SCREEN = {
  enviar: 'enviar',
  pago: 'pago',
  semPerguntas: 'sem-perguntas',
  resumo: 'resumo',
  composicao: 'conhecer:composicao',
  energia: 'conhecer:energia',
  lembrete: 'conhecer:lembrete',
  comparar: 'conhecer:comparar',
  escolher: 'contestar:escolher',
  texto: 'contestar:texto',
  onde: 'contestar:onde',
  ligar: 'contestar:ligar',
} as const

export const questionKey = (id: string) => `pergunta:${id}`
export const findingKey = (id: string) => `achado:${id}`

/** Tem algo indevido ou suspeito? Só isso abre a contestação e o lembrete. */
export const hasProblems = (a: Analysis) => a.findings.some((f) => f.severity !== 'ok')

/**
 * Monta a lista de telas a partir da análise. Tudo o que decide o que aparece vem da análise
 * (perguntas e achados já decididos pelo servidor); aqui só se organiza a ordem.
 */
export function buildScreens(analysis: Analysis | null): Screen[] {
  if (!analysis) {
    return [{ key: SCREEN.enviar, step: 'enviar', title: 'Vamos conferir sua fatura' }]
  }

  const problems = analysis.findings.filter((f) => f.severity !== 'ok')
  const screens: Screen[] = [{ key: SCREEN.enviar, step: 'enviar', title: 'Sua fatura foi enviada' }]

  // 2. Confirmar: uma pergunta por tela e, por último, "já pagou?" (só faz sentido com problemas).
  const nq = analysis.questions.length
  analysis.questions.forEach((q, i) => {
    screens.push({ key: questionKey(q.id), step: 'confirmar', title: `Pergunta ${i + 1} de ${nq}` })
  })
  if (problems.length > 0) screens.push({ key: SCREEN.pago, step: 'confirmar', title: 'Você já pagou esta fatura?' })
  if (nq === 0 && problems.length === 0) {
    screens.push({ key: SCREEN.semPerguntas, step: 'confirmar', title: 'Nenhuma pergunta por enquanto' })
  }

  // 3. Resultado: um resumo e um problema por tela.
  screens.push({ key: SCREEN.resumo, step: 'resultado', title: 'Veja o resultado' })
  problems.forEach((f, i) => {
    screens.push({ key: findingKey(f.id), step: 'resultado', title: `Problema ${i + 1} de ${problems.length}` })
  })

  // 4. Conhecer a conta: extras, cada um numa tela, e todos puláveis.
  if (composition(analysis.bill).length > 0) {
    screens.push({ key: SCREEN.composicao, step: 'conhecer', title: 'Conheça sua conta' })
  }
  if (analysis.bill.energy) screens.push({ key: SCREEN.energia, step: 'conhecer', title: 'Como foi o seu consumo' })
  if (problems.length > 0) screens.push({ key: SCREEN.lembrete, step: 'conhecer', title: 'Quer um lembrete por e-mail?' })
  screens.push({ key: SCREEN.comparar, step: 'conhecer', title: 'Quer comparar com outra fatura?' })

  // 5. Contestar: só se houver o que contestar.
  if (problems.length > 0) {
    screens.push(
      { key: SCREEN.escolher, step: 'contestar', title: 'O que você quer contestar?' },
      { key: SCREEN.texto, step: 'contestar', title: 'Seu texto está pronto' },
      { key: SCREEN.onde, step: 'contestar', title: 'Se a empresa não resolver' },
      { key: SCREEN.ligar, step: 'contestar', title: 'Prefere resolver por telefone?' },
    )
  }
  return screens
}

/** Número da etapa e total de etapas, contando só as que existem (sem achados, não há "Contestar"). */
export function stepProgress(screens: Screen[], current: Screen, hasAnalysis: boolean): { n: number; total: number } {
  if (!hasAnalysis) return { n: 1, total: DEFAULT_TOTAL }
  const steps = [...new Set(screens.map((s) => s.step))]
  return { n: steps.indexOf(current.step) + 1, total: steps.length }
}

/** Quem chega com o código da análise já enviou a fatura: começa nas perguntas. */
export function firstScreenKey(screens: Screen[], hasAnalysis: boolean): string {
  return hasAnalysis && screens.length > 1 ? screens[1].key : screens[0].key
}
