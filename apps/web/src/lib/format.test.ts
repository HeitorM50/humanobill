import { describe, expect, it } from 'vitest'
import { formatBRL, formatMonth } from './format'

describe('formatBRL', () => {
  it('formata centavos em reais', () => {
    expect(formatBRL(1290).replace(/\s/g, ' ')).toBe('R$ 12,90')
    expect(formatBRL(123456).replace(/\s/g, ' ')).toBe('R$ 1.234,56')
    expect(formatBRL(0).replace(/\s/g, ' ')).toBe('R$ 0,00')
  })
  it('formata o mês de referência', () => {
    expect(formatMonth('2026-10')).toBe('outubro de 2026')
  })
})
