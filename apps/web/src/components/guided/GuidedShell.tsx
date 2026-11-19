import { useEffect, useRef, type ReactNode } from 'react'
import { Link } from 'react-router'
import { Button, ButtonLink, Page } from '../ui'

/** Botões grandes (56 px ou mais, mesmo sem o A+), um embaixo do outro no celular. */
const BIG = 'min-h-14! w-full px-6 text-h4 sm:w-auto sm:min-w-44'

type Next =
  | { label: string; onClick: () => void; disabled?: boolean }
  | { label: string; to: string }
  | null

/**
 * Moldura de uma tela do modo passo a passo: "Passo X de Y" em texto, uma barra só de enfeite,
 * o título (que recebe o foco quando a tela muda) e os botões "Voltar" e "Próximo".
 */
export function GuidedShell({
  step,
  total,
  stepName,
  title,
  moveFocus,
  back,
  next,
  fullHref,
  children,
}: {
  step: number
  total: number
  stepName: string
  title: string
  /** `true` depois da primeira navegação: a tela nova leva o foco para o título. */
  moveFocus: boolean
  back: { onClick: () => void } | { to: string }
  next: Next
  /** Para onde vai o link "Ver a versão completa". */
  fullHref: string
  children: ReactNode
}) {
  const heading = useRef<HTMLHeadingElement>(null)
  // Só ao abrir a tela: cada tela tem sua própria `key`, então o foco não "puxa" a cada render.
  const focusOnOpen = useRef(moveFocus)
  useEffect(() => {
    if (focusOnOpen.current) heading.current?.focus()
  }, [])

  return (
    <Page width="form" className="space-y-6">
      <div className="space-y-2">
        <p className="flex flex-wrap items-baseline gap-x-3">
          <span data-testid="passo" className="text-h4 font-bold">{`Passo ${step} de ${total}`}</span>
          <span className="text-muted">{stepName}</span>
        </p>
        {/* Barra só ilustrativa: o texto acima já diz onde a pessoa está. */}
        <div aria-hidden="true" className="flex gap-1">
          {Array.from({ length: total }, (_, i) => (
            <span key={i} className={`h-2 flex-1 rounded-full ${i < step ? 'bg-accent' : 'bg-line'}`} />
          ))}
        </div>
      </div>

      <h1 ref={heading} tabIndex={-1} className="text-h1">{title}</h1>

      {/* Botões e caixas de marcar com 56 px também fora do A+ (pais e avós): vale para tudo dentro da tela. */}
      <div className="space-y-6 [&_button]:min-h-14 [&_label:has(input[type=checkbox])]:min-h-14">{children}</div>

      {/* Fixa na base da janela: em tela baixa (ou com A+) "Voltar" e "Próximo" não somem abaixo da dobra. */}
      <div data-guided-bar className="sticky bottom-0 z-10 -mx-5 flex flex-col gap-3 border-t border-line bg-bg px-5 py-3 sm:flex-row sm:justify-between md:-mx-10 md:px-10">
        {'to' in back ? (
          <ButtonLink to={back.to} variant="secondary" className={BIG}>Voltar</ButtonLink>
        ) : (
          <Button variant="secondary" className={BIG} onClick={back.onClick}>Voltar</Button>
        )}
        {next &&
          ('to' in next ? (
            <ButtonLink to={next.to} className={BIG}>{next.label}</ButtonLink>
          ) : (
            <Button className={BIG} disabled={next.disabled} onClick={next.onClick}>{next.label}</Button>
          ))}
      </div>

      <p>
        <Link to={fullHref} className="inline-flex min-h-12 items-center font-semibold text-accent underline underline-offset-4">
          Ver a versão completa
        </Link>
      </p>
    </Page>
  )
}
