import { Fragment, useMemo } from 'react'
import { Link } from 'react-router'
import type { GlossaryTerm } from '../../lib/glossary'
import { findTermsIn } from '../../lib/glossary'
import { Icon } from '../ui'
import { GLOSSARY_ATTR } from './useTermBalloon'


/**
 * Descrição do item com os termos do glossário viram botões. O texto continua inteiro
 * (o leitor de tela lê a frase toda); o botão só expõe o estado aberto/fechado.
 */
export function GlossaryText({
  text,
  openId,
  onToggle,
  balloonId,
}: {
  text: string
  /** Id do termo com o balão aberto, se for um dos desta descrição. */
  openId: string | null
  onToggle: (id: string) => void
  balloonId: string
}) {
  const matches = useMemo(() => findTermsIn(text), [text])
  const parts: React.ReactNode[] = []
  let cursor = 0
  for (const m of matches) {
    if (m.start > cursor) parts.push(<Fragment key={`t${cursor}`}>{text.slice(cursor, m.start)}</Fragment>)
    const open = openId === m.term.id
    parts.push(
      <button
        key={`b${m.start}`}
        type="button"
        {...{ [GLOSSARY_ATTR]: m.term.id }}
        aria-expanded={open}
        aria-controls={open ? balloonId : undefined}
        onClick={() => onToggle(m.term.id)}
        className="transicao-controle -my-3 inline-flex min-h-11.5 cursor-help items-center rounded-control px-0.5 text-left underline decoration-dotted decoration-2 underline-offset-4 hover:bg-accent-soft aria-expanded:bg-accent-soft"
      >
        {m.text}
      </button>,
    )
    cursor = m.end
  }
  if (cursor < text.length) parts.push(<Fragment key={`t${cursor}`}>{text.slice(cursor)}</Fragment>)
  return <>{parts}</>
}

/** O balão: a definição curta e o caminho para o glossário. Aparece logo abaixo da descrição. */
export function TermBalloon({ id, term }: { id: string; term: GlossaryTerm }) {
  return (
    <div
      id={id}
      role="group"
      aria-label={`O que é ${term.term}`}
      {...{ [GLOSSARY_ATTR]: 'balao' }}
      className="grid gap-1 rounded-card border border-accent bg-accent-soft px-4 py-3 text-body font-normal text-ink"
    >
      <p>
        <strong className="font-bold">{term.term}.</strong> {term.short}
      </p>
      <Link
        to={`/glossario#${term.id}`}
        className="inline-flex min-h-11.5 items-center gap-1.5 justify-self-start font-semibold text-accent underline underline-offset-4 hover:text-accent-strong"
      >
        Ver no glossário
        <Icon name="chevron" size={16} />
      </Link>
    </div>
  )
}
