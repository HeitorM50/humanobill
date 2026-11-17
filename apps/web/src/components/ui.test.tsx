import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { Alert, Button, Field } from './ui'

describe('Button', () => {
  it('a variante padrão é primary e o tipo padrão é button', () => {
    render(<Button>Analisar</Button>)
    const b = screen.getByRole('button', { name: 'Analisar' })
    expect(b).toHaveAttribute('type', 'button')
    expect(b.className).toContain('bg-accent')
    expect(b).not.toHaveAttribute('aria-busy')
  })

  it('loading: desativa, marca aria-busy e mostra o rótulo de espera', () => {
    render(<Button loading>Salvar</Button>)
    const b = screen.getByRole('button', { name: 'Aguarde' })
    expect(b).toBeDisabled()
    expect(b).toHaveAttribute('aria-busy', 'true')
    // O rótulo original continua no DOM (invisível) para segurar a largura.
    expect(b).toHaveTextContent('Salvar')
  })
})

describe('Alert', () => {
  it('tem ícone além da cor e repassa o role', () => {
    render(<Alert tone="danger" role="alert" title="Não deu certo">Tente de novo.</Alert>)
    const a = screen.getByRole('alert')
    expect(a).toHaveTextContent('Não deu certo')
    expect(a).toHaveTextContent('Tente de novo.')
    expect(a.querySelector('svg[aria-hidden="true"]')).not.toBeNull()
  })
})

describe('Field', () => {
  it('liga o rótulo, a ajuda e o erro ao campo', () => {
    render(
      <Field label="E-mail" hint="Só para o lembrete." error="Digite um e-mail com @.">
        {(c) => <input {...c} type="email" />}
      </Field>,
    )
    const input = screen.getByLabelText('E-mail')
    expect(input).toBeInvalid()
    expect(input).toHaveAccessibleDescription('Só para o lembrete. Digite um e-mail com @.')
  })

  it('sem erro, o campo não fica inválido', () => {
    render(<Field label="Operadora">{(c) => <input {...c} />}</Field>)
    const input = screen.getByLabelText('Operadora')
    expect(input).not.toHaveAttribute('aria-invalid')
    expect(input).not.toHaveAttribute('aria-describedby')
  })
})
