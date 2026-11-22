import { useState, type CSSProperties } from 'react'
import { formatBRL } from '../../lib/format'
import type { Analysis } from '../../types/Analysis'
import type { BBox } from '../../types/BBox'
import type { BillItem } from '../../types/BillItem'
import { billImageSrc } from './billImage'

/** Um achado com caixa na página 1, numerado na ordem da lista. */
type Entry = { n: number; id: string; title: string; amount: number; itemIds: string[]; boxes: BBox[] }
/** Uma caixa por item marcado (um item pode estar em mais de um achado). */
type Mark = { itemId: string; label: string; box: BBox; numbers: number[] }

// Só a página 1 tem imagem (PNG do exemplo ou foto enviada).
const PAGE = 1

const pct = (v: number) => `${(v * 100).toFixed(3)}%`

function boxStyle(b: BBox): CSSProperties {
  return { left: pct(b.x), top: pct(b.y), width: pct(b.w), height: pct(b.h) }
}

/** Junta as caixas de um achado e dá uma folga em volta (uma linha acima e
 * uma abaixo, para dar contexto), sem sair da página. */
function cropOf(boxes: BBox[]): BBox {
  const padY = Math.max(0.015, Math.max(...boxes.map((b) => b.h)) * 1.1)
  const x0 = Math.max(0, Math.min(...boxes.map((b) => b.x)) - 0.02)
  const y0 = Math.max(0, Math.min(...boxes.map((b) => b.y)) - padY)
  const x1 = Math.min(1, Math.max(...boxes.map((b) => b.x + b.w)) + 0.02)
  const y1 = Math.min(1, Math.max(...boxes.map((b) => b.y + b.h)) + padY)
  return { page: PAGE, x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
}

/** Escala mínima do trecho ampliado em relação à imagem original: abaixo disso,
 * o texto fica ilegível no celular e o trecho passa a rolar para o lado. */
const ZOOM_MIN_SCALE = 0.8

type MarkedItem = BillItem & { bbox: BBox }

function prefersReducedMotion(): boolean {
  return typeof window.matchMedia === 'function' && window.matchMedia('(prefers-reduced-motion: reduce)').matches
}

function buildEntries(analysis: Analysis): { entries: Entry[]; marks: Mark[] } {
  const items = new Map(analysis.bill.items.map((i) => [i.id, i]))
  const entries: Entry[] = []
  const marks = new Map<string, Mark>()
  for (const f of analysis.findings) {
    if (f.severity === 'ok') continue
    const withBox = f.item_refs
      .map((id) => items.get(id))
      .filter((i): i is MarkedItem => i?.bbox?.page === PAGE)
    if (!withBox.length) continue
    const n = entries.length + 1
    entries.push({ n, id: f.id, title: f.title, amount: f.amount_cents, itemIds: withBox.map((i) => i.id), boxes: withBox.map((i) => i.bbox) })
    for (const i of withBox) {
      const m = marks.get(i.id)
      if (m) m.numbers.push(n)
      else marks.set(i.id, { itemId: i.id, label: `${i.description}, ${formatBRL(i.amount_cents)}`, box: i.bbox, numbers: [n] })
    }
  }
  return { entries, marks: [...marks.values()] }
}

function Badge({ children, active, small }: { children: React.ReactNode; active?: boolean; small?: boolean }) {
  return (
    <span
      aria-hidden="true"
      className={`grid shrink-0 place-items-center rounded-full px-1 leading-none font-bold text-white ${
        small ? 'h-4 min-w-4 text-[0.625rem]' : 'h-6 min-w-6 text-caption'
      } ${active ? 'bg-accent' : 'bg-ink'}`}
    >
      {children}
    </span>
  )
}

/** Trecho ampliado do achado escolhido, recortado da mesma imagem. */
function Zoom({ src, entry, size }: { src: string; entry: Entry; size: { w: number; h: number } }) {
  const c = cropOf(entry.boxes)
  const style: CSSProperties = {
    aspectRatio: `${c.w * size.w} / ${c.h * size.h}`,
    backgroundImage: `url("${src}")`,
    backgroundSize: `${100 / c.w}% auto`,
    backgroundPosition: `${c.w < 1 ? (c.x / (1 - c.w)) * 100 : 0}% ${c.h < 1 ? (c.y / (1 - c.h)) * 100 : 0}%`,
  }
  // No celular o trecho fica com largura mínima legível e rola para o lado.
  const minWidth = `${Math.round(c.w * size.w * ZOOM_MIN_SCALE)}px`
  return (
    // Focável para dar para rolar pelo teclado quando o trecho é mais largo que a tela.
    <div
      role="img"
      aria-label={`Trecho ampliado da fatura: ${entry.title}`}
      tabIndex={0}
      className="overflow-x-auto rounded-card border border-line bg-surface"
    >
      <div aria-hidden="true" className="relative w-full bg-no-repeat" style={{ ...style, minWidth }}>
        {entry.boxes.map((b, i) => (
          <span
            key={i}
            className="absolute rounded-sm bg-highlight/70 mix-blend-multiply"
            style={boxStyle({ ...b, x: (b.x - c.x) / c.w, y: (b.y - c.y) / c.h, w: b.w / c.w, h: b.h / c.h })}
          />
        ))}
      </div>
    </div>
  )
}

/**
 * Marcação visual (#51): a imagem da fatura com marca-texto amarelo sobre os
 * itens que têm achado. Sem imagem ou sem caixa, não mostra nada.
 */
export function BillOverlay({ analysis }: { analysis: Analysis }) {
  const src = billImageSrc(analysis)
  const { entries, marks } = buildEntries(analysis)
  const [activeId, setActiveId] = useState<string | null>(null)
  const [size, setSize] = useState<{ w: number; h: number } | null>(null)
  const [failed, setFailed] = useState(false)

  if (!src || failed || entries.length === 0) return null

  const active = entries.find((e) => e.id === activeId) ?? entries[0]
  const activeItems = new Set(active.itemIds)
  // Corta o branco do fim da página: mostra até um pouco abaixo da última marca.
  const bottom = Math.min(1, Math.max(...marks.map((m) => m.box.y + m.box.h)) + 0.2)
  const cropStyle = size && bottom < 0.9 ? { aspectRatio: `${size.w} / ${size.h * bottom}` } : undefined

  const choose = (e: Entry) => {
    setActiveId(e.id)
    const el = document.getElementById(`marca-${analysis.id}-${e.itemIds[0]}`)
    el?.scrollIntoView?.({ block: 'nearest', behavior: prefersReducedMotion() ? 'auto' : 'smooth' })
  }

  return (
    <section aria-labelledby="marcacao-t" className="space-y-4">
      <h2 id="marcacao-t" className="text-h2">Onde está na sua fatura</h2>
      <p className="max-w-text">
        Marquei em amarelo as cobranças com problema. Toque num item da lista para ver o trecho de perto.
      </p>

      <div className="grid gap-6 md:grid-cols-[minmax(0,1fr)_minmax(0,22rem)] md:items-start">
        <div className="min-w-0 space-y-4">
          <ol className="space-y-2">
            {entries.map((e) => {
              const on = e.id === active.id
              return (
                <li key={e.id}>
                  <button
                    type="button"
                    aria-pressed={on}
                    onClick={() => choose(e)}
                    className={`flex min-h-11.5 w-full items-center gap-3 rounded-control border-2 px-3 py-2 text-left transition-colors duration-150 ${
                      on ? 'border-accent bg-accent-soft' : 'border-line bg-surface hover:bg-accent-soft'
                    }`}
                  >
                    <Badge active={on}>{e.n}</Badge>
                    <span className="min-w-0 flex-1 font-semibold">{e.title}</span>
                    <span className="shrink-0">{formatBRL(e.amount)}</span>
                    <span className="sr-only"> — mostrar na fatura</span>
                  </button>
                </li>
              )
            })}
          </ol>
          {size && <Zoom src={src} entry={active} size={size} />}
        </div>

        <figure className="min-w-0">
          <div className="overflow-hidden rounded-card border border-line bg-surface" style={cropStyle}>
            <div className="relative">
              <img
                src={src}
                alt={`Imagem da sua fatura com ${marks.length === 1 ? 'uma cobrança marcada' : `${marks.length} cobranças marcadas`} em amarelo`}
                className="block h-auto w-full"
                onLoad={(ev) => setSize({ w: ev.currentTarget.naturalWidth || 1, h: ev.currentTarget.naturalHeight || 1 })}
                onError={() => setFailed(true)}
              />
              {marks.map((m) => {
                const on = activeItems.has(m.itemId)
                return (
                  <div
                    key={m.itemId}
                    id={`marca-${analysis.id}-${m.itemId}`}
                    role="img"
                    aria-label={`Marca ${m.numbers.join(' e ')}: ${m.label}`}
                    className={`absolute rounded-sm mix-blend-multiply transition-colors duration-150 ${
                      on ? 'bg-highlight outline-2 outline-offset-1 outline-accent' : 'bg-highlight/50'
                    }`}
                    style={boxStyle(m.box)}
                  >
                    {/* O número fica à esquerda da caixa; colada na borda da página, vai para dentro. */}
                    <span className={`absolute top-1/2 -translate-y-1/2 ${m.box.x < 0.06 ? 'left-0.5' : '-left-0.5 -translate-x-full'}`}>
                      <Badge active={on} small>{m.numbers.join(',')}</Badge>
                    </span>
                  </div>
                )
              })}
            </div>
          </div>
          <figcaption className="mt-2 text-caption text-muted">Os números da imagem são os mesmos da lista.</figcaption>
        </figure>
      </div>
    </section>
  )
}
