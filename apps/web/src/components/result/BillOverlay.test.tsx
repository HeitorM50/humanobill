import { fireEvent, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import energia1 from '../../../../../fixtures/extracted/energia-1.json'
import telecom1 from '../../../../../fixtures/extracted/telecom-1.json'
import type { Analysis } from '../../types/Analysis'
import type { ExtractedBill } from '../../types/ExtractedBill'
import type { Finding } from '../../types/Finding'
import { BillOverlay } from './BillOverlay'
import { forgetUploadedBills, rememberUploadedBill } from './billImage'

const achado = (rule: string, refs: string[], title: string, cents: number, severity: Finding['severity'] = 'suspeito'): Finding => ({
  id: `${rule}:${refs.join('+')}`,
  rule_id: rule,
  severity,
  title,
  explanation: 'Explicação curta.',
  item_refs: refs,
  original_text: null,
  page: 1,
  amount_cents: cents,
  legal_basis: [],
})

function analise(bill: ExtractedBill, findings: Finding[], source: Analysis['source'] = 'demo'): Analysis {
  return {
    id: 'a1',
    created_at: '2026-11-01T10:00:00Z',
    expires_at: '2026-12-01T10:00:00Z',
    source,
    bill,
    answers: {},
    paid: null,
    findings,
    questions: [],
    recoverable_cents: 0,
    rules_version: 'teste',
  }
}

const achadosTelecom1 = [
  achado('TEL-01', ['i2'], 'Clube de música que você pode não ter pedido', 1290),
  achado('TEL-01', ['i3'], 'Seguro que você pode não ter pedido', 990),
]

const semCaixas = (b: ExtractedBill): ExtractedBill => ({ ...b, items: b.items.map(({ bbox: _ignorada, ...i }) => i) })

/** Dispara o onLoad da imagem com o tamanho do PNG dos exemplos. */
function carregaImagem() {
  const img = screen.getByRole('img', { name: /Imagem da sua fatura/ }) as HTMLImageElement
  Object.defineProperty(img, 'naturalWidth', { value: 900 })
  Object.defineProperty(img, 'naturalHeight', { value: 1300 })
  fireEvent.load(img)
  return img
}

describe('BillOverlay', () => {
  beforeEach(() => {
    Element.prototype.scrollIntoView = vi.fn()
  })
  afterEach(() => {
    forgetUploadedBills()
    vi.restoreAllMocks()
  })

  it('no exemplo, mostra o PNG público com uma marca rotulada por item com achado', () => {
    render(<BillOverlay analysis={analise(telecom1 as ExtractedBill, achadosTelecom1)} />)
    expect(screen.getByRole('heading', { name: 'Onde está na sua fatura' })).toBeInTheDocument()
    expect(carregaImagem()).toHaveAttribute('src', '/exemplos/telecom-1.png')
    expect(screen.getByRole('img', { name: /^Marca 1: Clube Vivaz Música, R\$\s12,90$/ })).toBeInTheDocument()
    expect(screen.getByRole('img', { name: /^Marca 2: Seguro Tela, R\$\s9,90$/ })).toBeInTheDocument()
    // O plano não tem achado: não é marcado.
    expect(screen.queryByRole('img', { name: /Plano/ })).not.toBeInTheDocument()
    // A caixa sai do bbox da fixture (normalizado 0–1 -> %).
    const marca = screen.getByRole('img', { name: /^Marca 1:/ })
    expect(parseFloat(marca.style.left)).toBeCloseTo(7.44)
    expect(parseFloat(marca.style.top)).toBeCloseTo(27.73)
  })

  it('a lista de texto é outra forma de achar o item: clicar realça e rola até a caixa', async () => {
    render(<BillOverlay analysis={analise(telecom1 as ExtractedBill, achadosTelecom1)} />)
    carregaImagem()
    const [b1, b2] = screen.getAllByRole('button', { name: /mostrar na fatura/ })
    expect(b1).toHaveAccessibleName(/Clube de música.*R\$\s12,90/)
    expect(b1).toHaveAttribute('aria-pressed', 'true')
    expect(b2).toHaveAttribute('aria-pressed', 'false')
    expect(screen.getByRole('img', { name: /Trecho ampliado da fatura: Clube de música/ })).toBeInTheDocument()

    await userEvent.click(b2)
    expect(b2).toHaveAttribute('aria-pressed', 'true')
    expect(b1).toHaveAttribute('aria-pressed', 'false')
    const marca2 = screen.getByRole('img', { name: /^Marca 2:/ })
    expect(Element.prototype.scrollIntoView).toHaveBeenCalledTimes(1)
    expect(vi.mocked(Element.prototype.scrollIntoView).mock.contexts[0]).toBe(marca2)
    expect(marca2.className).toContain('outline-accent')
    expect(screen.getByRole('img', { name: /Trecho ampliado da fatura: Seguro/ })).toBeInTheDocument()
  })

  it('item em dois achados vira uma marca só, com os dois números', () => {
    const findings = [
      achado('ENE-02', ['i1', 'i2'], 'Conta bem acima da sua média', 25920),
      achado('ENE-03', ['i1', 'i2'], 'Leitura estimada', 25920),
    ]
    render(<BillOverlay analysis={analise(energia1 as ExtractedBill, findings)} />)
    expect(screen.getByRole('img', { name: /^Marca 1 e 2: Uso da rede \(TUSD\)/ })).toBeInTheDocument()
    expect(screen.getAllByRole('img', { name: /^Marca/ })).toHaveLength(2)
    expect(screen.getAllByRole('button', { name: /mostrar na fatura/ })).toHaveLength(2)
  })

  it('sem bbox nos itens, a seção não aparece', () => {
    const { container } = render(<BillOverlay analysis={analise(semCaixas(telecom1 as ExtractedBill), achadosTelecom1)} />)
    expect(container).toBeEmptyDOMElement()
  })

  it('sem achado com problema, a seção não aparece', () => {
    const ok = [achado('TEL-01', ['i2'], 'Tudo certo', 0, 'ok')]
    const { container } = render(<BillOverlay analysis={analise(telecom1 as ExtractedBill, ok)} />)
    expect(container).toBeEmptyDOMElement()
  })

  it('upload sem imagem guardada (ou PDF) não mostra nada', () => {
    const a = analise(telecom1 as ExtractedBill, achadosTelecom1, 'upload')
    rememberUploadedBill('a1', new File(['%PDF'], 'fatura.pdf', { type: 'application/pdf' }))
    const { container } = render(<BillOverlay analysis={a} />)
    expect(container).toBeEmptyDOMElement()
  })

  it('upload de foto usa o arquivo que ficou no navegador', () => {
    const create = vi.fn(() => 'blob:fatura-local')
    const revoke = vi.fn()
    Object.assign(URL, { createObjectURL: create, revokeObjectURL: revoke })
    rememberUploadedBill('a1', new File(['png'], 'foto.png', { type: 'image/png' }))
    render(<BillOverlay analysis={analise(telecom1 as ExtractedBill, achadosTelecom1, 'upload')} />)
    expect(screen.getByRole('img', { name: /Imagem da sua fatura/ })).toHaveAttribute('src', 'blob:fatura-local')
    // Guardar outra foto libera a anterior da memória.
    rememberUploadedBill('a2', new File(['png'], 'outra.png', { type: 'image/png' }))
    expect(revoke).toHaveBeenCalledWith('blob:fatura-local')
  })

  it('se a imagem não carrega, a seção some', () => {
    const { container } = render(<BillOverlay analysis={analise(telecom1 as ExtractedBill, achadosTelecom1)} />)
    fireEvent.error(screen.getByRole('img', { name: /Imagem da sua fatura/ }))
    expect(container).toBeEmptyDOMElement()
  })

  it('fatura editada (total diferente do exemplo) não usa o PNG do exemplo', () => {
    const editada = { ...(telecom1 as ExtractedBill), total_cents: 1 }
    const { container } = render(<BillOverlay analysis={analise(editada, achadosTelecom1)} />)
    expect(container).toBeEmptyDOMElement()
  })
})
