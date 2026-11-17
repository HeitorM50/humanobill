const SHAPE = 'M6 5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v24l-4-3-3 3-3-3-3 3-3-3-4 3z'

/**
 * Logo "recibo grifado": símbolo de 28 px + a palavra em texto vivo.
 * `negative` é a versão para fundo azul (símbolo branco, linhas azuis).
 * O símbolo é decorativo: quem dá o nome é a palavra ao lado.
 */
export function Logo({ negative = false, className = '' }: { negative?: boolean; className?: string }) {
  const body = negative ? 'fill-on-accent' : 'fill-accent'
  const lines = negative ? 'fill-accent' : 'fill-on-accent'
  return (
    <span
      className={`inline-flex items-center gap-2 font-display text-xl font-bold tracking-display ${negative ? 'text-on-accent' : 'text-accent'} ${className}`}
    >
      <svg viewBox="0 0 32 32" aria-hidden="true" className="size-7 shrink-0">
        <path className={body} d={SHAPE} />
        <rect x="10" y="7.5" width="12" height="2.5" rx="1" className={lines} />
        <rect x="8.5" y="12.5" width="15" height="5.5" rx="1" className="fill-highlight" transform="rotate(-3 16 15)" />
        <rect x="10" y="20.5" width="8" height="2.5" rx="1" className={lines} />
      </svg>
      HumanoBill
    </span>
  )
}
