import { formatBRL } from '../lib/format'

/** Campo de moeda BRL: o usuário digita só números e o valor vira centavos. */
export function MoneyInput({ id, value, onChange, className, ...rest }: { id: string; value: number | null; onChange: (cents: number | null) => void; className?: string } & Omit<React.InputHTMLAttributes<HTMLInputElement>, 'value' | 'onChange'>) {
  return (
    <input
      {...rest}
      id={id}
      type="text"
      inputMode="numeric"
      autoComplete="off"
      placeholder="R$ 0,00"
      value={value == null ? '' : formatBRL(value)}
      onChange={(e) => {
        const digits = e.target.value.replace(/\D/g, '').slice(0, 9)
        onChange(digits ? Number(digits) : null)
      }}
      // Dentro de um `Field`, quem chama passa o `className` do controle (estado de erro incluído).
      className={className ?? 'min-h-12 w-full rounded-control border border-field-line bg-surface px-3.5 py-2.5 text-body'}
    />
  )
}
