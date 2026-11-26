import { useEffect, useId, type RefObject } from 'react'

/** Marca o que faz parte do balão ou dos termos: tocar neles não conta como "clique fora". */
export const GLOSSARY_ATTR = 'data-glossary'

/**
 * Comportamento de balão acessível: fecha com Esc (devolvendo o foco ao termo) e com
 * clique fora (sem roubar o foco do que foi clicado).
 */
export function useDismissBalloon(open: boolean, close: () => void, scope: RefObject<HTMLElement | null>) {
  useEffect(() => {
    if (!open) return
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return
      scope.current?.querySelector<HTMLElement>('button[aria-expanded="true"]')?.focus()
      close()
    }
    const onPointer = (e: Event) => {
      if (e.target instanceof Element && e.target.closest(`[${GLOSSARY_ATTR}]`) && scope.current?.contains(e.target)) return
      close()
    }
    document.addEventListener('keydown', onKey)
    document.addEventListener('pointerdown', onPointer)
    return () => {
      document.removeEventListener('keydown', onKey)
      document.removeEventListener('pointerdown', onPointer)
    }
  }, [open, close, scope])
}

/** Id estável para ligar o botão ao balão por `aria-controls`. */
export function useBalloonId() {
  return `balao-${useId()}`
}
