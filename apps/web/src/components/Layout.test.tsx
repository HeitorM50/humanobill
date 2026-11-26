/// <reference types="node" />
import { readFileSync } from 'node:fs'
import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import Layout from './Layout'
import { Button, type ButtonVariant } from './ui'

// Lido do disco: com `css: false` o Vitest esvazia o CSS importado.
const css = readFileSync('src/index.css', 'utf8') // o Vitest roda a partir de apps/web
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

/** Altura mínima declarada nas classes do Tailwind (min-h-N, size-N ou valor em rem), em rem. */
function minHeightRem(className: string): number {
  const m =
    /(?:^|\s)min-h-(\d+(?:\.\d+)?)(?=\s|$)/.exec(className) ??
    /(?:^|\s)size-(\d+(?:\.\d+)?)(?=\s|$)/.exec(className)
  if (m) return Number(m[1]) * 0.25 // a escala do Tailwind v4 é N × 0,25 rem
  const rem = /(?:^|\s)(?:min-h|size)-\[(\d+(?:\.\d+)?)rem\](?=\s|$)/.exec(className)
  if (rem) return Number(rem[1])
  throw new Error(`sem altura mínima nas classes: ${className}`)
}

/** Tamanho (px) da base quando o A+ está ligado, lido do CSS do app (16 px é o padrão do navegador). */
function largeRootPx(): number {
  const rule = /html\.texto-grande\s*\{[^}]*font-size:\s*(\d+(?:\.\d+)?)%/.exec(css)
  if (!rule) throw new Error('regra html.texto-grande sem font-size em %')
  return (16 * Number(rule[1])) / 100
}

describe('RF20: tamanhos no modo A+', () => {
  it('o texto base passa a ter pelo menos 20 px', () => {
    const bodyRem = Number(/--text-body:\s*(\d+(?:\.\d+)?)rem/.exec(css)?.[1])
    expect(largeRootPx()).toBeGreaterThanOrEqual(20)
    expect(bodyRem * largeRootPx()).toBeGreaterThanOrEqual(20)
  })

  it.each(['primary', 'secondary', 'tertiary', 'ghost', 'onDark'] as ButtonVariant[])('o botão %s passa a ter pelo menos 56 px de altura', (variant) => {
    render(<Button variant={variant}>Teste</Button>)
    const btn = screen.getByRole('button', { name: 'Teste' })
    expect(minHeightRem(btn.className) * largeRootPx()).toBeGreaterThanOrEqual(56)
  })

  it('o próprio botão A+ passa a ter pelo menos 56 px de altura', () => {
    expect(minHeightRem(mount().className) * largeRootPx()).toBeGreaterThanOrEqual(56)
  })
})

describe('navegação', () => {
  it('mantém o link "Ir para o conteúdo" e o alvo dele', () => {
    mount()
    expect(screen.getByRole('link', { name: 'Ir para o conteúdo' })).toHaveAttribute('href', '#conteudo')
    expect(screen.getByRole('main')).toHaveAttribute('id', 'conteudo')
  })
})

describe('Minhas contas: link de navegação', () => {
  it('aparece no rodapé', () => {
    mount()
    const footer = screen.getByRole('navigation', { name: 'Rodapé' })
    expect(within(footer).getByRole('link', { name: 'Minhas contas' })).toHaveAttribute('href', '/minhas-contas')
  })

  it('some quando o navegador não deixa guardar a lista', () => {
    const blocked = () => {
      throw new DOMException('bloqueado', 'SecurityError')
    }
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(blocked)
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(blocked)
    mount()
    expect(screen.queryByRole('link', { name: 'Minhas contas' })).not.toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'Privacidade' })).toBeInTheDocument()
  })
})

describe('RF24: link do glossário', () => {
  it('aparece no rodapé e leva para /glossario', () => {
    mount()
    const footer = screen.getByRole('navigation', { name: 'Rodapé' })
    expect(within(footer).getByRole('link', { name: 'Glossário' })).toHaveAttribute('href', '/glossario')
  })
})
