import { render, screen, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import energia2 from '../../../../../fixtures/extracted/energia-2.json'
import telecom1 from '../../../../../fixtures/extracted/telecom-1.json'
import type { ExtractedBill } from '../../types/ExtractedBill'
import { Composition } from './Composition'

const norm = (s: string | null) => (s ?? '').replace(/\s/g, ' ')

describe('Composition', () => {
  it('mostra, em lista de texto, cada parte da conta de luz em reais e em porcentagem', () => {
    render(<Composition bill={energia2 as ExtractedBill} />)
    expect(screen.getByRole('heading', { name: 'Para onde vai seu dinheiro' })).toBeInTheDocument()
    const list = screen.getByRole('list')
    const rows = within(list).getAllByRole('listitem')
    expect(rows.map((r) => norm(r.textContent))).toEqual([
      expect.stringMatching(/^Energia.*R\$ 158,40.*68,4%/),
      expect.stringMatching(/^Impostos.*R\$ 35,64.*15,4%/),
      expect.stringMatching(/^Iluminação pública.*R\$ 14,30.*6,2%/),
      expect.stringMatching(/^Extras.*R\$ 12,90.*5,6%/),
      expect.stringMatching(/^Outros.*R\$ 10,34.*4,4%/),
    ])
  })

  it('mostra o total da fatura, igual à soma das partes', () => {
    render(<Composition bill={energia2 as ExtractedBill} />)
    expect(norm(screen.getByText(/^Total/).parentElement!.textContent)).toMatch(/R\$ 231,58.*100%/)
  })

  it('destaca Extras com a cor de alerta, ícone e texto (não só cor)', () => {
    const { container } = render(<Composition bill={telecom1 as ExtractedBill} />)
    const extras = screen.getByText('Extras').closest('li')!
    expect(extras).toHaveAttribute('data-alerta', 'true')
    expect(extras.querySelector('[data-cor]')).toHaveClass('bg-warning')
    expect(extras.querySelector('svg[aria-hidden="true"]')).not.toBeNull()
    expect(screen.getByText('Plano').closest('li')).not.toHaveAttribute('data-alerta')
    // O gráfico é só ilustração: a lista acima diz a mesma coisa.
    expect(container.querySelector('[data-grafico]')).toHaveAttribute('aria-hidden', 'true')
  })

  it('não mostra nada quando a fatura não tem valor', () => {
    const vazia = { ...(telecom1 as ExtractedBill), items: [], total_cents: 0 }
    const { container } = render(<Composition bill={vazia} />)
    expect(container).toBeEmptyDOMElement()
  })
})
