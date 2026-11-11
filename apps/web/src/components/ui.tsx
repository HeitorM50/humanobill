import type { ButtonHTMLAttributes, ReactNode } from 'react'
import { Link, type LinkProps } from 'react-router'
import { useServerWaking } from '../lib/api'

const base =
  'inline-flex min-h-12 items-center justify-center gap-2 rounded-control px-5 py-3 text-base font-semibold transition-colors disabled:cursor-not-allowed disabled:opacity-50'
const variants = {
  primary: 'bg-accent text-white shadow-card hover:bg-accent-strong',
  secondary: 'border border-line bg-surface text-ink hover:bg-accent-soft',
}
type Variant = keyof typeof variants

export function Button({ variant = 'primary', className = '', ...p }: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: Variant }) {
  return <button type="button" className={`${base} ${variants[variant]} ${className}`} {...p} />
}

export function ButtonLink({ variant = 'primary', className = '', ...p }: LinkProps & { variant?: Variant }) {
  return <Link className={`${base} ${variants[variant]} ${className}`} {...p} />
}

export function Card({ children, className = '' }: { children: ReactNode; className?: string }) {
  return <div className={`rounded-card border border-line bg-surface p-5 shadow-card ${className}`}>{children}</div>
}

export function ServerWakingBanner() {
  const waking = useServerWaking()
  if (!waking) return null
  return (
    <div role="status" className="bg-amber-100 px-4 py-2 text-center text-sm font-medium text-amber-900">
      Acordando o servidor… isso pode levar alguns segundos.
    </div>
  )
}
