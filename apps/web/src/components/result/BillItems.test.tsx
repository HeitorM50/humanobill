import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter } from 'react-router'
import { describe, expect, it } from 'vitest'
import { analysis } from '../../test/fixtures'
import type { BillItem } from '../../types/BillItem'
import type { ExtractedBill } from '../../types/ExtractedBill'
import { GLOSSARY } from '../../lib/glossary'
import { BillItems } from './BillItems'

const item = (id: string, description: string): BillItem => ({
  id,
  description,
  amount_cents: 1000,
  category: 'energia',
  period: null,
  original_text: description.toUpperCase(),
  page: 1,
  plain_explanation: 'Explicação do item.',
})

const bill = (...descriptions: string[]): ExtractedBill => ({
  ...analysis.bill,
  items: descriptions.map((d, i) => item(`x${i}`, d)),
})

function mount(b: ExtractedBill) {
  return render(
    <MemoryRouter>
      <BillItems bill={b} findings={[]} />
      <button type="button">Fora</button>
    </MemoryRouter>,
  )
}

const tusd = GLOSSARY.find((t) => t.id === 'tusd')!

describe('RF24: termo tocável nos itens da fatura', () => {
  it('Cenário: toco em "TUSD" e vejo a definição curta e o link para o glossário', async () => {
    mount(bill('Uso da rede (TUSD) 198 kWh'))
    const termo = screen.getByRole('button', { name: 'TUSD' })
    expect(termo).toHaveAttribute('aria-expanded', 'false')
    expect(screen.queryByText(tusd.short)).not.toBeInTheDocument()

    await userEvent.click(termo)
    expect(termo).toHaveAttribute('aria-expanded', 'true')
    expect(screen.getByText(tusd.short)).toBeInTheDocument()
    expect(screen.getByRole('link', { name: /Ver no glossário/ })).toHaveAttribute('href', '/glossario#tusd')
  })

  it('a descrição continua inteira para quem lê a tela', () => {
    mount(bill('Uso da rede (TUSD) 198 kWh'))
    expect(screen.getByRole('heading', { level: 3 })).toHaveTextContent('Uso da rede (TUSD) 198 kWh')
    expect(screen.getByRole('button', { name: 'kWh' })).toBeInTheDocument()
  })

  it('o parêntese não se separa do termo na quebra de linha', () => {
    mount(bill('Uso da rede (TUSD) 198 kWh'))
    const grupo = screen.getByRole('button', { name: 'TUSD' }).parentElement!
    expect(grupo).toHaveClass('whitespace-nowrap')
    expect(grupo).toHaveTextContent(/^\(TUSD\)$/)
  })

  it('não vira botão quando não há termo (e "TE" não casa em "Telecom")', () => {
    mount(bill('Plano Vivaz Controle 20GB', 'Vivaz Telecom'))
    expect(screen.queryByRole('button', { name: 'TE' })).not.toBeInTheDocument()
    // Só o botão "Fora" do teste.
    expect(screen.getAllByRole('button')).toHaveLength(1)
  })

  it('toca de novo e fecha', async () => {
    mount(bill('Uso da rede (TUSD) 198 kWh'))
    const termo = screen.getByRole('button', { name: 'TUSD' })
    await userEvent.click(termo)
    await userEvent.click(termo)
    expect(termo).toHaveAttribute('aria-expanded', 'false')
    expect(screen.queryByText(tusd.short)).not.toBeInTheDocument()
  })

  it('Esc fecha e devolve o foco ao termo', async () => {
    mount(bill('Uso da rede (TUSD) 198 kWh'))
    const termo = screen.getByRole('button', { name: 'TUSD' })
    await userEvent.click(termo)
    await userEvent.click(screen.getByRole('link', { name: /Ver no glossário/ }))
    await userEvent.keyboard('{Escape}')
    expect(screen.queryByText(tusd.short)).not.toBeInTheDocument()
    expect(termo).toHaveFocus()
  })

  it('clique fora fecha, sem roubar o foco do que foi clicado', async () => {
    mount(bill('Uso da rede (TUSD) 198 kWh'))
    await userEvent.click(screen.getByRole('button', { name: 'TUSD' }))
    const fora = screen.getByRole('button', { name: 'Fora' })
    await userEvent.click(fora)
    expect(screen.queryByText(tusd.short)).not.toBeInTheDocument()
    expect(fora).toHaveFocus()
  })

  it('clicar dentro do balão não fecha', async () => {
    mount(bill('Uso da rede (TUSD) 198 kWh'))
    await userEvent.click(screen.getByRole('button', { name: 'TUSD' }))
    await userEvent.click(screen.getByText(tusd.short))
    expect(screen.getByText(tusd.short)).toBeInTheDocument()
  })

  it('abrir outro termo troca o balão', async () => {
    mount(bill('Uso da rede (TUSD) 198 kWh'))
    await userEvent.click(screen.getByRole('button', { name: 'TUSD' }))
    await userEvent.click(screen.getByRole('button', { name: 'kWh' }))
    expect(screen.queryByText(tusd.short)).not.toBeInTheDocument()
    const kwh = GLOSSARY.find((t) => t.id === 'kwh')!
    expect(screen.getByText(kwh.short)).toBeInTheDocument()
    expect(screen.getByRole('link', { name: /Ver no glossário/ })).toHaveAttribute('href', '/glossario#kwh')
  })

  it('só um balão aberto por vez na lista', async () => {
    mount(bill('Uso da rede (TUSD) 198 kWh', 'ICMS 18%'))
    await userEvent.click(screen.getByRole('button', { name: 'TUSD' }))
    await userEvent.click(screen.getByRole('button', { name: 'ICMS' }))
    expect(screen.getAllByRole('link', { name: /Ver no glossário/ })).toHaveLength(1)
    expect(screen.getByRole('link', { name: /Ver no glossário/ })).toHaveAttribute('href', '/glossario#icms')
  })
})
