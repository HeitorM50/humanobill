import { describe, expect, it } from 'vitest'
import energia1 from '../../../../fixtures/extracted/energia-1.json'
import energia2 from '../../../../fixtures/extracted/energia-2.json'
import energia3 from '../../../../fixtures/extracted/energia-3.json'
import telecom1 from '../../../../fixtures/extracted/telecom-1.json'
import telecom2 from '../../../../fixtures/extracted/telecom-2.json'
import type { BillItem } from '../types/BillItem'
import type { ExtractedBill } from '../types/ExtractedBill'
import type { ItemCategory } from '../types/ItemCategory'
import { composition } from './derive'

const item = (id: string, category: ItemCategory, amount_cents: number): BillItem => ({
  id,
  description: `Item ${id}`,
  amount_cents,
  category,
  period: null,
  original_text: `ITEM ${id}`,
  page: 1,
  plain_explanation: 'Explicação.',
})

const bill = (kind: ExtractedBill['kind'], items: BillItem[], total_cents?: number): ExtractedBill => ({
  kind,
  provider_name: 'Luzia Energia',
  reference_month: '2026-10',
  due_date: '2026-11-10',
  issue_date: null,
  total_cents: total_cents ?? items.reduce((a, i) => a + i.amount_cents, 0),
  plan_name: null,
  items,
  late_fees: null,
  energy: null,
  extraction_confidence: 0.95,
})

const sumCents = (parts: ReturnType<typeof composition>) => parts.reduce((a, p) => a + p.cents, 0)
const sumPermille = (parts: ReturnType<typeof composition>) => parts.reduce((a, p) => a + p.permille, 0)

describe('composition', () => {
  it('divide a conta de luz de R$ 200,00 e a soma das partes é igual ao total', () => {
    const b = bill('energy', [
      item('i1', 'energia', 6000),
      item('i2', 'energia', 5000),
      item('i3', 'imposto', 4500),
      item('i4', 'bandeira', 1000),
      item('i5', 'iluminacao_publica', 1500),
      item('i6', 'outros', 2000),
    ])
    expect(b.total_cents).toBe(20000)
    const parts = composition(b)
    expect(parts.map((p) => [p.key, p.label, p.cents, p.permille])).toEqual([
      ['principal', 'Energia', 11000, 550],
      ['impostos', 'Impostos', 4500, 225],
      ['bandeira', 'Bandeira', 1000, 50],
      ['iluminacao', 'Iluminação pública', 1500, 75],
      ['outros', 'Outros', 2000, 100],
    ])
    expect(sumCents(parts)).toBe(b.total_cents)
    expect(sumPermille(parts)).toBe(1000)
  })

  it('usa "Plano" no lugar de "Energia" no telecom', () => {
    const parts = composition(telecom1 as ExtractedBill)
    expect(parts[0]).toMatchObject({ key: 'principal', label: 'Plano', cents: 9990 })
  })

  it.each([
    ['energia-1', energia1],
    ['energia-2', energia2],
    ['energia-3', energia3],
    ['telecom-1', telecom1],
    ['telecom-2', telecom2],
  ])('fixture %s: soma em centavos = total e porcentagem fecha em 100%%', (_nome, json) => {
    const b = json as ExtractedBill
    const parts = composition(b)
    expect(sumCents(parts)).toBe(b.total_cents)
    expect(sumPermille(parts)).toBe(1000)
    expect(parts.every((p) => p.cents > 0)).toBe(true)
  })

  it('agrupa multa, juros e itens "outros" em Outros', () => {
    const parts = composition(telecom2 as ExtractedBill)
    expect(parts.map((p) => [p.key, p.cents])).toEqual([
      ['principal', 12990],
      ['outros', 960],
    ])
  })

  it('não deixa a porcentagem passar de 100% quando o arredondamento é ingrato', () => {
    // Três partes iguais: 33,3% + 33,3% + 33,3% = 99,9%. A sobra vai para uma delas.
    const b = bill('energy', [item('i1', 'energia', 10000), item('i2', 'imposto', 10000), item('i3', 'bandeira', 10000)])
    const parts = composition(b)
    expect(sumPermille(parts)).toBe(1000)
    expect(Math.max(...parts.map((p) => p.permille))).toBeLessThanOrEqual(334)
  })

  it('mostra a parte pequena com décimo de ponto, sem virar 0%', () => {
    const parts = composition(energia3 as ExtractedBill)
    const bandeira = parts.find((p) => p.key === 'bandeira')
    expect(bandeira).toMatchObject({ cents: 91, permille: 4 })
  })

  it('destaca Extras (servico_adicional) como alerta e só eles', () => {
    const parts = composition(telecom1 as ExtractedBill)
    const extras = parts.find((p) => p.key === 'extras')
    expect(extras).toMatchObject({ label: 'Extras', cents: 1290 + 990, alert: true })
    expect(parts.filter((p) => p.alert).map((p) => p.key)).toEqual(['extras'])
  })

  it('omite categorias sem valor', () => {
    const parts = composition(bill('telecom', [item('i1', 'plano', 9990)]))
    expect(parts.map((p) => p.key)).toEqual(['principal'])
    expect(parts[0].permille).toBe(1000)
  })

  it('manda para Outros a diferença entre a soma dos itens e o total da fatura', () => {
    const b = bill('energy', [item('i1', 'energia', 9990), item('i2', 'imposto', 5000)], 15000)
    const parts = composition(b)
    expect(sumCents(parts)).toBe(15000)
    expect(parts.find((p) => p.key === 'outros')).toMatchObject({ cents: 10 })
  })

  it('desconto (valor negativo) abate da categoria e a soma continua igual ao total', () => {
    const b = bill('telecom', [item('i1', 'plano', 10000), item('i2', 'outros', -1000)])
    const parts = composition(b)
    expect(sumCents(parts)).toBe(9000)
    expect(sumPermille(parts)).toBe(1000)
    expect(parts.map((p) => p.key)).toEqual(['principal'])
  })

  it('fatura sem valor devolve lista vazia', () => {
    expect(composition(bill('telecom', []))).toEqual([])
    expect(composition(bill('telecom', [item('i1', 'plano', 0)]))).toEqual([])
  })
})
