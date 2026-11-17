import { useId, type ComponentPropsWithRef, type HTMLAttributes, type ReactNode } from 'react'
import { Link, type LinkProps } from 'react-router'
import { useServerWaking } from '../lib/api'

/* ---------- Ícones (guia 5.9): traço arredondado, currentColor, sempre decorativos ---------- */

const ICONS = {
  lock: (
    <>
      <rect x="5" y="11" width="14" height="9" rx="2" />
      <path d="M8 11V8a4 4 0 0 1 8 0v3" />
    </>
  ),
  check: <path d="M5 12.5l4.5 4.5L19 7" />,
  close: <path d="M6 6l12 12M18 6L6 18" />,
  danger: (
    <>
      <circle cx="12" cy="12" r="9.5" />
      <path d="M9 9l6 6M15 9l-6 6" />
    </>
  ),
  warning: (
    <>
      <path d="M12 3.5l9.5 16.5h-19z" />
      <path d="M12 10v4.5M12 17.4v.1" />
    </>
  ),
  ok: (
    <>
      <circle cx="12" cy="12" r="9.5" />
      <path d="M7.8 12.4l2.9 2.9 5.5-6" />
    </>
  ),
  info: (
    <>
      <circle cx="12" cy="12" r="9.5" />
      <path d="M12 11v5.5M12 7.5v.1" />
    </>
  ),
  external: <path d="M14 4h6v6M20 4l-9 9M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5" />,
  chevron: <path d="M9 6l6 6-6 6" />,
}
export type IconName = keyof typeof ICONS

// Em rem, para o modo A+ aumentar o ícone junto com o texto.
const ICON_SIZES = { 16: 'size-4', 20: 'size-5', 24: 'size-6' }

export function Icon({ name, size = 20, className = '' }: { name: IconName; size?: keyof typeof ICON_SIZES; className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={2.2}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      className={`shrink-0 ${ICON_SIZES[size]} ${className}`}
    >
      {ICONS[name]}
    </svg>
  )
}

/** Spinner de 16 px. Com `prefers-reduced-motion` fica parado: o rótulo ao lado diz o estado. */
export function Spinner({ className = '' }: { className?: string }) {
  return (
    <span
      aria-hidden="true"
      className={`inline-block size-4 shrink-0 rounded-full border-2 border-current border-r-transparent motion-safe:animate-spin ${className}`}
    />
  )
}

/* ---------- Botões (guia 5.1) ---------- */

const buttonBase =
  'transicao-controle inline-flex min-h-12 items-center justify-center gap-2 rounded-control border-[1.5px] py-2.5 text-body font-semibold not-disabled:active:translate-y-px disabled:cursor-not-allowed disabled:opacity-45'

// O espaço lateral fica na variante (o Ghost usa 10 px) para não depender da ordem do CSS.
const buttonVariants = {
  primary: 'border-transparent bg-accent px-5 text-on-accent not-disabled:hover:bg-accent-strong not-disabled:active:bg-accent-strong',
  secondary: 'border-accent bg-surface px-5 text-accent not-disabled:hover:bg-accent-soft not-disabled:active:bg-accent-soft',
  /** Destaque: a ação principal sobre bloco azul (some `foco-sobre-azul` nesse caso) e "Copiar". */
  tertiary: 'border-transparent bg-highlight px-5 text-on-highlight not-disabled:hover:bg-highlight-strong not-disabled:active:bg-highlight-strong',
  ghost: 'border-transparent px-2.5 text-accent underline-offset-4 not-disabled:hover:bg-accent-soft not-disabled:hover:underline',
  /** Contorno sobre azul: a ação alternativa dentro de um bloco `accent`. */
  onDark: 'foco-sobre-azul border-on-accent px-5 text-on-accent not-disabled:hover:bg-on-accent/15 not-disabled:active:bg-on-accent/15',
}
export type ButtonVariant = keyof typeof buttonVariants

const buttonClass = (variant: ButtonVariant, className: string) => `${buttonBase} ${buttonVariants[variant]} ${className}`

type ButtonProps = ComponentPropsWithRef<'button'> & {
  variant?: ButtonVariant
  /** Mostra o spinner com `loadingLabel`, desativa o botão e marca `aria-busy`. */
  loading?: boolean
  loadingLabel?: string
}

export function Button({ variant = 'primary', loading = false, loadingLabel = 'Aguarde', className = '', disabled, children, ...p }: ButtonProps) {
  return (
    <button type="button" className={buttonClass(variant, className)} disabled={disabled || loading} aria-busy={loading || undefined} {...p}>
      {loading ? (
        // As duas camadas ocupam a mesma célula: o rótulo original segura a largura e só o de espera aparece.
        <span className="inline-grid place-items-center">
          <span aria-hidden="true" className="invisible col-start-1 row-start-1 inline-flex items-center gap-2">{children}</span>
          <span className="col-start-1 row-start-1 inline-flex items-center gap-2">
            <Spinner />
            {loadingLabel}
          </span>
        </span>
      ) : (
        children
      )}
    </button>
  )
}

export function ButtonLink({ variant = 'primary', className = '', ...p }: LinkProps & { variant?: ButtonVariant }) {
  return <Link className={buttonClass(variant, className)} {...p} />
}

/* ---------- Superfícies (guia 5.2) ---------- */

type DivProps = HTMLAttributes<HTMLDivElement>

/** Cartão: borda de 1 px, sem sombra. */
export function Card({ className = '', ...p }: DivProps) {
  return <div className={`rounded-card border border-line bg-surface p-5 ${className}`} {...p} />
}

/** Papel: a fatura. É a única superfície com `shadow-paper`. */
export function Paper({ className = '', ...p }: DivProps) {
  return <div className={`rounded-card border border-line bg-surface text-ink shadow-paper ${className}`} {...p} />
}

/** Cabeçalho da fatura, com o divisor tracejado. */
export function PaperHeader({ className = '', ...p }: DivProps) {
  return <div className={`flex justify-between gap-3 border-b border-dashed border-line px-4 py-3 font-bold ${className}`} {...p} />
}

/**
 * Linha da fatura: descrição à esquerda, valor à direita. `highlight` passa o marca-texto nos dois.
 * `children` é a anotação de caneta, abaixo da linha. Use dentro de uma lista (`<ul>`/`<ol>`).
 */
export function PaperLine({
  label,
  value,
  highlight = false,
  className = '',
  children,
}: {
  label: ReactNode
  value: ReactNode
  highlight?: boolean
  className?: string
  children?: ReactNode
}) {
  const mark = highlight ? 'marca-texto' : ''
  return (
    <li className={`grid gap-2 border-b border-line px-4 py-3 ${className}`}>
      <div className="flex justify-between gap-3 font-semibold">
        <span className="min-w-0 break-words">
          <span className={mark}>{label}</span>
        </span>
        <span className="whitespace-nowrap">
          <span className={mark}>{value}</span>
        </span>
      </div>
      {children && <p className="border-l-2 border-accent pl-3 text-caption text-accent">{children}</p>}
    </li>
  )
}

/** Linha de total da fatura. */
export function PaperTotal({ className = '', ...p }: DivProps) {
  return <div className={`flex justify-between gap-3 px-4 py-3 text-h4 font-bold ${className}`} {...p} />
}

const ALERT_TONES = {
  danger: { box: 'border-danger-line bg-danger-soft', text: 'text-danger', icon: 'danger' },
  warning: { box: 'border-warning-line bg-warning-soft', text: 'text-warning', icon: 'warning' },
  ok: { box: 'border-ok-line bg-ok-soft', text: 'text-ok', icon: 'ok' },
  info: { box: 'border-info-line bg-info-soft', text: 'text-info', icon: 'info' },
} as const
export type AlertTone = keyof typeof ALERT_TONES

/**
 * Alerta: ícone + texto, nunca só cor. Passe `role="alert"` (erro) ou `role="status"` (aviso)
 * quando o conteúdo aparece depois de uma ação.
 */
export function Alert({ tone, title, className = '', children, ...p }: Omit<DivProps, 'title'> & { tone: AlertTone; title?: ReactNode }) {
  const t = ALERT_TONES[tone]
  return (
    <div className={`flex gap-3 rounded-card border p-4 ${t.box} ${className}`} {...p}>
      {/* A caixa tem a altura de uma linha de texto, então o ícone fica alinhado com a primeira linha. */}
      <span className={`flex h-[1.5em] shrink-0 items-center ${t.text}`}>
        <Icon name={t.icon} />
      </span>
      <div className="min-w-0 flex-1 space-y-3 break-words">
        {title && <p className={`font-bold ${t.text}`}>{title}</p>}
        {children}
      </div>
    </div>
  )
}

/* ---------- Formulário (guia 5.3) ---------- */

export type FieldControlProps = {
  id: string
  className: string
  'aria-invalid'?: true
  'aria-describedby'?: string
}

/**
 * Campo: rótulo sempre visível, ajuda opcional e mensagem de erro ligada por `aria-describedby`.
 * O controle é quem chama que desenha, para servir a `<input>`, `<select>` e `MoneyInput`:
 *
 *   <Field label="Operadora" error={erro}>{(c) => <input {...c} value={v} onChange={…} />}</Field>
 */
export function Field({
  label,
  hint,
  error,
  className = '',
  children,
}: {
  label: ReactNode
  hint?: ReactNode
  /** Diz como corrigir. Quando presente, o campo fica em estado de erro. */
  error?: ReactNode
  className?: string
  children: (control: FieldControlProps) => ReactNode
}) {
  const id = useId()
  const hintId = hint ? `${id}-ajuda` : undefined
  const errorId = error ? `${id}-erro` : undefined
  const describedBy = [hintId, errorId].filter(Boolean).join(' ') || undefined
  const control: FieldControlProps = {
    id,
    className: `transicao-controle min-h-12 w-full rounded-control border px-3.5 py-2.5 text-body text-ink disabled:cursor-not-allowed disabled:opacity-60 ${
      error ? 'border-danger bg-danger-soft' : 'border-field-line bg-surface'
    }`,
    'aria-invalid': error ? true : undefined,
    'aria-describedby': describedBy,
  }
  return (
    <div className={`grid gap-2 ${className}`}>
      <label htmlFor={id} className="font-semibold">{label}</label>
      {children(control)}
      {hint && <p id={hintId} className="text-caption text-muted">{hint}</p>}
      {error && <p id={errorId} className="text-caption font-semibold text-danger">{error}</p>}
    </div>
  )
}

/* ---------- Layout ---------- */

const PAGE_WIDTHS = { page: 'max-w-page', form: 'max-w-2xl' }

/**
 * Container de página: margem lateral de 20 px (40 px a partir de `md`), largura máxima do guia
 * e espaço vertical de 32 px (56 px no desktop). O `<main>` não limita a largura.
 */
export function Page({ width = 'page', className = '', ...p }: DivProps & { width?: keyof typeof PAGE_WIDTHS }) {
  return <div className={`mx-auto w-full px-5 py-8 md:px-10 md:py-14 ${PAGE_WIDTHS[width]} ${className}`} {...p} />
}

/** Faixa de "servidor acordando" (cold start do plano gratuito). */
export function ServerWakingBanner() {
  const waking = useServerWaking()
  if (!waking) return null
  return (
    <div role="status" className="border-b border-warning-line bg-warning-soft text-warning">
      <p className="mx-auto flex max-w-page items-center gap-2 px-5 py-2 text-caption font-semibold md:px-10">
        <Spinner />
        Acordando o servidor… isso pode levar alguns segundos.
      </p>
    </div>
  )
}
