const brl = new Intl.NumberFormat('pt-BR', { style: 'currency', currency: 'BRL' })

/** Centavos -> "R$ 1.234,56". */
export function formatBRL(cents: number): string {
  return brl.format(cents / 100)
}

/** "2026-10" -> "outubro de 2026". Devolve o texto original se inválido. */
export function formatMonth(ym: string): string {
  const m = /^(\d{4})-(0[1-9]|1[0-2])$/.exec(ym)
  if (!m) return ym
  const date = new Date(Date.UTC(Number(m[1]), Number(m[2]) - 1, 1))
  return new Intl.DateTimeFormat('pt-BR', { month: 'long', year: 'numeric', timeZone: 'UTC' }).format(date)
}
