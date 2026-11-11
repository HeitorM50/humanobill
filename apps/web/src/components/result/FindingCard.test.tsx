import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import { analysis, finding } from '../../test/fixtures'
import { FindingCard } from './FindingCard'
import { SeverityBadge } from './SeverityBadge'

describe('FindingCard', () => {
  it('mostra trecho original, página e base legal com link externo', () => {
    render(<FindingCard finding={finding} items={[analysis.bill.items[1]]} />)
    expect(screen.getByText('CLUBE VZ MUS 12,90')).toBeInTheDocument()
    expect(screen.getByText(/página 1/)).toBeInTheDocument()
    const link = screen.getByRole('link', { name: /CDC art\. 39, III/ })
    expect(link).toHaveAttribute('href', finding.legal_basis[0].url)
    expect(link).toHaveAttribute('target', '_blank')
    expect(link).toHaveAttribute('rel', expect.stringContaining('noopener'))
    expect(screen.getByText('Clube Vivaz Música')).toBeInTheDocument()
  })

  it('mostra o motivo em linguagem simples (até 25 palavras, sem juridiquês)', () => {
    render(<FindingCard finding={finding} items={[]} />)
    expect(screen.getByText(finding.explanation)).toBeInTheDocument()
    expect(finding.explanation.split(/\s+/).length).toBeLessThanOrEqual(25)
    expect(finding.explanation).not.toMatch(/supramencionad|outrossim|ensejar/i)
  })

  it('explica a diferença entre IA e regra em "Por que isso foi marcado?"', async () => {
    render(<FindingCard finding={finding} items={[]} />)
    const btn = screen.getByRole('button', { name: 'Por que isso foi marcado?' })
    expect(btn).toHaveAttribute('aria-expanded', 'false')
    await userEvent.click(btn)
    expect(btn).toHaveAttribute('aria-expanded', 'true')
    expect(screen.getByText(/A IA só leu sua fatura\. Quem marcou foi a regra TEL-01, que compara com a lei\./)).toBeInTheDocument()
  })
})

describe('SeverityBadge', () => {
  it.each([
    ['indevido', 'Indevido'],
    ['suspeito', 'Suspeito — confira'],
    ['ok', 'Tudo certo'],
  ] as const)('%s tem texto, não só cor', (sev, label) => {
    const { container } = render(<SeverityBadge severity={sev} />)
    expect(screen.getByText(label)).toBeInTheDocument()
    expect(container.querySelector('svg[aria-hidden="true"]')).not.toBeNull()
  })
})
