import type { Analysis } from '../types/Analysis'
import type { Severity } from '../types/Severity'

/**
 * Texto do botão "Ouvir" (RF21). Aqui só se monta a fala a partir do que a análise já traz:
 * quem decidiu o que é indevido foi o motor de regras. Nada é recalculado nem reclassificado.
 */

/** Como cada severidade é dita. Vírgula e ponto fazem a voz respirar. */
const SPOKEN_SEVERITY: Record<Severity, string> = {
  indevido: 'Indevido',
  suspeito: 'Suspeito, confira',
  ok: 'Tudo certo',
}

const reaisFormat = new Intl.NumberFormat('pt-BR')

/** Centavos -> "12 reais e 90 centavos". A voz lê isso melhor do que "R$ 12,90". */
export function spokenBRL(cents: number): string {
  const total = Math.max(0, Math.round(cents))
  const reais = Math.floor(total / 100)
  const centavos = total % 100
  if (total === 0) return 'zero reais'
  const parts: string[] = []
  if (reais > 0) parts.push(`${reaisFormat.format(reais)} ${reais === 1 ? 'real' : 'reais'}`)
  if (centavos > 0) parts.push(`${centavos} ${centavos === 1 ? 'centavo' : 'centavos'}`)
  return parts.join(' e ')
}

/** Garante um ponto final só, para a voz fazer a pausa entre as falas. */
const sentence = (text: string) => {
  const t = text.trim()
  return /[.!?]$/.test(t) ? t : `${t}.`
}

/**
 * Falas do resultado, na ordem: total da fatura, valor que parece indevido (ou para conferir)
 * e cada achado. Uma fala por item, porque falas curtas são mais confiáveis nos navegadores
 * do que uma só longa, e deixam parar entre um achado e outro.
 */
export function buildSpeech(analysis: Analysis): string[] {
  const { bill, findings } = analysis
  const lines = [`Fatura da ${bill.provider_name}, no total de ${spokenBRL(bill.total_cents)}.`]

  const sumOf = (severity: Severity) => {
    const list = findings.filter((f) => f.severity === severity)
    return { count: list.length, cents: list.reduce((t, f) => t + f.amount_cents, 0) }
  }
  const undue = sumOf('indevido')
  const suspect = sumOf('suspeito')
  const charges = (n: number) => `${n} ${n === 1 ? 'cobrança' : 'cobranças'}`

  if (undue.count > 0) lines.push(`Parecem indevidos ${spokenBRL(undue.cents)}, em ${charges(undue.count)}.`)
  else if (suspect.count > 0) lines.push(`${spokenBRL(suspect.cents)} para conferir, em ${charges(suspect.count)}.`)
  else lines.push('Não achei nada fora do lugar.')

  for (const f of findings) {
    const label = SPOKEN_SEVERITY[f.severity]
    const title = f.severity === 'ok' ? `${f.title}.` : `${f.title}, ${spokenBRL(f.amount_cents)}.`
    lines.push(`${label}. ${title} ${sentence(f.explanation)}`)
  }
  return lines
}

/** O navegador tem síntese de voz? Sem ela, o botão "Ouvir" não aparece. */
export function isSpeechSupported(): boolean {
  return 'speechSynthesis' in globalThis && 'SpeechSynthesisUtterance' in globalThis
}

