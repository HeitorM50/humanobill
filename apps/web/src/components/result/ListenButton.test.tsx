import { act, cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { buildSpeech } from '../../lib/speech'
import { analysis } from '../../test/fixtures'
import { ListenButton } from './ListenButton'

/** Fala falsa: guarda o texto e deixa o teste disparar o fim ou o erro, como o navegador faz. */
class FakeUtterance {
  lang = ''
  onend: (() => void) | null = null
  onerror: ((e: { error: string }) => void) | null = null
  text: string
  constructor(text: string) {
    this.text = text
  }
}

let queue: FakeUtterance[]
const synth = {
  speak: vi.fn((u: FakeUtterance) => void queue.push(u)),
  // Como no navegador: cancelar interrompe a fila e avisa cada fala com `onerror`.
  cancel: vi.fn(() => {
    const pending = queue
    queue = []
    pending.forEach((u) => u.onerror?.({ error: 'canceled' }))
  }),
}

beforeEach(() => {
  queue = []
  synth.speak.mockClear()
  synth.cancel.mockClear()
})
afterEach(() => {
  cleanup() // desmonta antes de tirar a voz falsa, senão o desmonte já não a encontra
  vi.unstubAllGlobals()
})

const withSpeech = () => {
  vi.stubGlobal('speechSynthesis', synth)
  vi.stubGlobal('SpeechSynthesisUtterance', FakeUtterance)
}

describe('RF21: ListenButton', () => {
  it('não aparece em navegador sem síntese de voz', () => {
    const { container } = render(<ListenButton analysis={analysis} />)
    expect(container).toBeEmptyDOMElement()
  })

  it('mostra "Ouvir" e, ao clicar, lê em pt-BR cada fala e vira "Parar"', async () => {
    withSpeech()
    render(<ListenButton analysis={analysis} />)
    await userEvent.click(screen.getByRole('button', { name: 'Ouvir' }))

    const expected = buildSpeech(analysis)
    expect(synth.speak).toHaveBeenCalledTimes(expected.length)
    expect(queue.map((u) => u.text)).toEqual(expected)
    expect(queue.every((u) => u.lang === 'pt-BR')).toBe(true)
    expect(screen.getByRole('button', { name: 'Parar' })).toBeInTheDocument()
  })

  it('volta a "Ouvir" quando a última fala termina', async () => {
    withSpeech()
    render(<ListenButton analysis={analysis} />)
    await userEvent.click(screen.getByRole('button', { name: 'Ouvir' }))
    act(() => queue.at(-1)?.onend?.())
    expect(screen.getByRole('button', { name: 'Ouvir' })).toBeInTheDocument()
  })

  it('"Parar" cancela a leitura e volta a "Ouvir"', async () => {
    withSpeech()
    render(<ListenButton analysis={analysis} />)
    await userEvent.click(screen.getByRole('button', { name: 'Ouvir' }))
    await userEvent.click(screen.getByRole('button', { name: 'Parar' }))
    expect(synth.cancel).toHaveBeenCalled()
    expect(screen.getByRole('button', { name: 'Ouvir' })).toBeInTheDocument()
  })

  it('volta a "Ouvir" se a voz der erro', async () => {
    withSpeech()
    render(<ListenButton analysis={analysis} />)
    await userEvent.click(screen.getByRole('button', { name: 'Ouvir' }))
    act(() => queue[0].onerror?.({ error: 'synthesis-failed' }))
    expect(screen.getByRole('button', { name: 'Ouvir' })).toBeInTheDocument()
  })

  it('para a leitura ao sair da tela', async () => {
    withSpeech()
    const { unmount } = render(<ListenButton analysis={analysis} />)
    await userEvent.click(screen.getByRole('button', { name: 'Ouvir' }))
    synth.cancel.mockClear()
    unmount()
    expect(synth.cancel).toHaveBeenCalledTimes(1)
  })

  it('não mexe na voz do navegador ao sair da tela se não estava lendo', () => {
    withSpeech()
    const { unmount } = render(<ListenButton analysis={analysis} />)
    unmount()
    expect(synth.cancel).not.toHaveBeenCalled()
  })
})
