import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import Layout from './Layout'

const KEY = 'humanobill:texto-grande'

function mount() {
  vi.stubGlobal('fetch', vi.fn(() => Promise.resolve(new Response('{}'))))
  render(
    <MemoryRouter>
      <Routes>
        <Route element={<Layout />}>
          <Route index element={<h1>Início</h1>} />
        </Route>
      </Routes>
    </MemoryRouter>,
  )
  return screen.getByRole('button', { name: 'Aumentar o texto' })
}

beforeEach(() => {
  window.localStorage.clear()
  document.documentElement.className = ''
})
afterEach(() => {
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
})

describe('RF20: botão A+', () => {
  it('liga e desliga o texto grande e guarda a escolha', async () => {
    const aplus = mount()
    expect(aplus).toHaveAttribute('aria-pressed', 'false')

    await userEvent.click(aplus)
    expect(aplus).toHaveAttribute('aria-pressed', 'true')
    expect(document.documentElement).toHaveClass('texto-grande')
    expect(window.localStorage.getItem(KEY)).toBe('1')

    await userEvent.click(aplus)
    expect(aplus).toHaveAttribute('aria-pressed', 'false')
    expect(document.documentElement).not.toHaveClass('texto-grande')
    expect(window.localStorage.getItem(KEY)).toBe('0')
  })

  it('volta ligado quando a escolha já estava salva', () => {
    window.localStorage.setItem(KEY, '1')
    expect(mount()).toHaveAttribute('aria-pressed', 'true')
    expect(document.documentElement).toHaveClass('texto-grande')
  })

  it('funciona mesmo com o localStorage bloqueado', async () => {
    const blocked = () => {
      throw new DOMException('bloqueado', 'SecurityError')
    }
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(blocked)
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(blocked)

    const aplus = mount()
    expect(aplus).toHaveAttribute('aria-pressed', 'false')
    await userEvent.click(aplus)
    expect(aplus).toHaveAttribute('aria-pressed', 'true')
    expect(document.documentElement).toHaveClass('texto-grande')
  })
})

describe('navegação', () => {
  it('mantém o link "Ir para o conteúdo" e o alvo dele', () => {
    mount()
    expect(screen.getByRole('link', { name: 'Ir para o conteúdo' })).toHaveAttribute('href', '#conteudo')
    expect(screen.getByRole('main')).toHaveAttribute('id', 'conteudo')
  })
})
