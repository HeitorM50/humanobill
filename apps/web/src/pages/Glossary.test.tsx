import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes, useLocation } from 'react-router'
import { describe, expect, it } from 'vitest'
import { GLOSSARY } from '../lib/glossary'
import Glossary from './Glossary'

function Where() {
  const { search } = useLocation()
  return <p data-testid="search">{search}</p>
}

function mount(path = '/glossario') {
  render(
    <MemoryRouter initialEntries={[path]}>
      <Routes>
        <Route
          path="glossario"
          element={
            <>
              <Glossary />
              <Where />
            </>
          }
        />
      </Routes>
    </MemoryRouter>,
  )
}

describe('RF24: página do glossário', () => {
  it('lista todos os termos, cada um com id para âncora', () => {
    mount()
    expect(screen.getByRole('heading', { level: 1, name: 'Glossário' })).toBeInTheDocument()
    for (const t of GLOSSARY) {
      const el = document.getElementById(t.id)
      expect(el).not.toBeNull()
      expect(within(el!).getByRole('heading', { name: t.term })).toBeInTheDocument()
    }
    expect(document.getElementById('tusd')).toHaveAttribute('tabindex', '-1')
  })

  it('o campo de busca tem rótulo visível', () => {
    mount()
    expect(screen.getByLabelText('Buscar um termo')).toBeInTheDocument()
  })

  it('Cenário: busca "bandeira" e vê as bandeiras verde, amarela e vermelha', async () => {
    mount()
    await userEvent.type(screen.getByLabelText('Buscar um termo'), 'bandeira')
    const bandeira = document.getElementById('bandeira')!
    expect(bandeira).toBeInTheDocument()
    expect(within(bandeira).getByText(/verde/i)).toBeInTheDocument()
    expect(within(bandeira).getByText(/amarela/i)).toBeInTheDocument()
    expect(within(bandeira).getByText(/vermelha/i)).toBeInTheDocument()
    expect(document.getElementById('anuidade')).toBeNull()
  })

  it('a busca ignora acento e caixa', async () => {
    mount()
    await userEvent.type(screen.getByLabelText('Buscar um termo'), 'ILUMINACAO')
    expect(document.getElementById('cip')).not.toBeNull()
    expect(document.getElementById('icms')).toBeNull()
  })

  it('avisa quando nenhum termo é encontrado e oferece limpar a busca', async () => {
    mount()
    await userEvent.type(screen.getByLabelText('Buscar um termo'), 'xyzzy')
    expect(screen.getByRole('status')).toHaveTextContent('Nenhum termo encontrado')
    expect(document.getElementById('tusd')).toBeNull()
    await userEvent.click(screen.getByRole('button', { name: 'Limpar a busca' }))
    expect(document.getElementById('tusd')).not.toBeNull()
    expect(screen.getByLabelText('Buscar um termo')).toHaveValue('')
  })

  it('lê ?q= da URL e escreve a busca de volta nela', async () => {
    mount('/glossario?q=bandeira')
    expect(screen.getByLabelText('Buscar um termo')).toHaveValue('bandeira')
    expect(document.getElementById('bandeira')).not.toBeNull()
    expect(document.getElementById('anuidade')).toBeNull()

    await userEvent.clear(screen.getByLabelText('Buscar um termo'))
    await userEvent.type(screen.getByLabelText('Buscar um termo'), 'kwh')
    expect(screen.getByTestId('search')).toHaveTextContent('?q=kwh')
    await userEvent.clear(screen.getByLabelText('Buscar um termo'))
    expect(screen.getByTestId('search')).toHaveTextContent('')
  })

  it('diz quantos termos aparecem', async () => {
    mount()
    expect(screen.getByRole('status')).toHaveTextContent(`${GLOSSARY.length} termos`)
    await userEvent.type(screen.getByLabelText('Buscar um termo'), 'anuidade')
    expect(screen.getByRole('status')).toHaveTextContent('1 termo')
  })
})
