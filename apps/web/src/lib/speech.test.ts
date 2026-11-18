import { afterEach, describe, expect, it, vi } from 'vitest'
import { analysis, finding } from '../test/fixtures'
import type { Analysis } from '../types/Analysis'
import type { Finding } from '../types/Finding'
import { buildSpeech, isSpeechSupported, spokenBRL } from './speech'

const undue: Finding = {
  ...finding,
  id: 'TEL-03:i4',
  rule_id: 'TEL-03',
  severity: 'indevido',
  title: 'Multa acima do limite',
  explanation: 'A multa passou de 2% do valor da fatura.',
  amount_cents: 523,
}
const withFindings = (findings: Finding[]): Analysis => ({ ...analysis, findings })

describe('spokenBRL', () => {
  it.each([
    [12980, '129 reais e 80 centavos'],
    [1290, '12 reais e 90 centavos'],
    [100, '1 real'],
    [101, '1 real e 1 centavo'],
    [50, '50 centavos'],
    [1, '1 centavo'],
    [0, 'zero reais'],
    [123456, '1.234 reais e 56 centavos'],
  ])('%i centavos vira "%s"', (cents, text) => {
    expect(spokenBRL(cents)).toBe(text)
  })
})

describe('buildSpeech', () => {
  it('lê o total, o valor que parece indevido e cada achado', () => {
    const lines = buildSpeech(withFindings([undue, finding]))
    expect(lines[0]).toContain('Vivaz Telecom')
    expect(lines[0]).toContain('129 reais e 80 centavos')
    expect(lines[1]).toBe('Parecem indevidos 5 reais e 23 centavos, em 1 cobrança.')
    expect(lines).toHaveLength(2 + 2)
    expect(lines[2]).toBe('Indevido. Multa acima do limite, 5 reais e 23 centavos. A multa passou de 2% do valor da fatura.')
    expect(lines[3]).toBe(`Suspeito, confira. ${finding.title}, 12 reais e 90 centavos. ${finding.explanation}`)
  })

  it('soma só os achados indevidos no valor que parece indevido', () => {
    const second = { ...undue, id: 'TEL-03:i5', amount_cents: 100 }
    const lines = buildSpeech(withFindings([undue, second, finding]))
    expect(lines[1]).toBe('Parecem indevidos 6 reais e 23 centavos, em 2 cobranças.')
  })

  it('sem achado indevido, fala o valor para conferir', () => {
    expect(buildSpeech(withFindings([finding]))[1]).toBe('12 reais e 90 centavos para conferir, em 1 cobrança.')
  })

  it('sem achados, diz que não achou nada e não lista achado', () => {
    const lines = buildSpeech(withFindings([]))
    expect(lines).toHaveLength(2)
    expect(lines[1]).toBe('Não achei nada fora do lugar.')
  })

  it('lê o achado "tudo certo" com o rótulo dele e sem valor', () => {
    const ok: Finding = { ...finding, id: 'X:i1', severity: 'ok', title: 'Plano conferido', explanation: 'O plano bate com o contrato.', amount_cents: 0 }
    const lines = buildSpeech(withFindings([ok]))
    expect(lines[1]).toBe('Não achei nada fora do lugar.')
    expect(lines[2]).toBe('Tudo certo. Plano conferido. O plano bate com o contrato.')
  })

  it('não repete ponto final quando a explicação já termina em ponto', () => {
    const lines = buildSpeech(withFindings([undue]))
    expect(lines.every((l) => !l.includes('..'))).toBe(true)
  })

  it('não altera a análise recebida', () => {
    const input = withFindings([undue, finding])
    const copy = structuredClone(input)
    buildSpeech(input)
    expect(input).toEqual(copy)
  })
})

describe('isSpeechSupported', () => {
  afterEach(() => vi.unstubAllGlobals())

  it('é falso sem speechSynthesis', () => {
    expect('speechSynthesis' in window).toBe(false) // o jsdom não tem
    expect(isSpeechSupported()).toBe(false)
  })

  it('é verdadeiro com speechSynthesis e SpeechSynthesisUtterance', () => {
    vi.stubGlobal('speechSynthesis', { speak: vi.fn(), cancel: vi.fn() })
    vi.stubGlobal('SpeechSynthesisUtterance', class {})
    expect(isSpeechSupported()).toBe(true)
  })

  it('é falso se só existir speechSynthesis, sem a classe da fala', () => {
    vi.stubGlobal('speechSynthesis', { speak: vi.fn(), cancel: vi.fn() })
    expect(isSpeechSupported()).toBe(false)
  })
})
