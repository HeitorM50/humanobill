import { useEffect, useId, useRef, useState } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { patchAnalysis } from '../../lib/api'
import { sumDiverges } from '../../lib/derive'
import { formatBRL } from '../../lib/format'
import type { Analysis } from '../../types/Analysis'
import type { ExtractedBill } from '../../types/ExtractedBill'
import type { ReadingType } from '../../types/ReadingType'
import { MoneyInput } from '../MoneyInput'
import { Button } from '../ui'

const READINGS: { value: ReadingType; label: string }[] = [
  { value: 'real', label: 'Leitura real' },
  { value: 'estimada', label: 'Leitura estimada' },
  { value: 'media', label: 'Pela média' },
]
const field = 'min-h-12 w-full rounded-control border border-line bg-surface px-4 text-base'
const FOCUSABLE = 'button:not([disabled]), input:not([disabled]), select:not([disabled]), a[href], [tabindex]:not([tabindex="-1"])'

function Drawer({ analysis, onClose }: { analysis: Analysis; onClose: () => void }) {
  const qc = useQueryClient()
  const uid = useId()
  const panel = useRef<HTMLDivElement>(null)
  const [bill, setBill] = useState<ExtractedBill>(analysis.bill)
  const diverges = sumDiverges(bill)
  const valid = bill.provider_name.trim() !== '' && bill.items.every((i) => i.description.trim() !== '')

  const save = useMutation({
    mutationFn: () => patchAnalysis(analysis.id, { bill }),
    onSuccess: (next) => {
      qc.setQueryData(['analysis', analysis.id], next)
      onClose()
    },
  })

  useEffect(() => {
    const prev = document.body.style.overflow
    document.body.style.overflow = 'hidden'
    panel.current?.querySelector<HTMLElement>(FOCUSABLE)?.focus()
    return () => {
      document.body.style.overflow = prev
    }
  }, [])

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') {
      e.stopPropagation()
      onClose()
      return
    }
    if (e.key !== 'Tab' || !panel.current) return
    const els = [...panel.current.querySelectorAll<HTMLElement>(FOCUSABLE)]
    if (!els.length) return
    const first = els[0]
    const last = els[els.length - 1]
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault()
      last.focus()
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault()
      first.focus()
    }
  }

  const setItem = (id: string, patch: Partial<ExtractedBill['items'][number]>) =>
    setBill((b) => ({ ...b, items: b.items.map((i) => (i.id === id ? { ...i, ...patch } : i)) }))

  return (
    <div className="fixed inset-0 z-50 flex justify-end bg-ink/50" onMouseDown={(e) => e.target === e.currentTarget && onClose()} onKeyDown={onKeyDown}>
      <div ref={panel} role="dialog" aria-modal="true" aria-labelledby={`${uid}-t`} className="flex h-full w-full max-w-lg flex-col bg-bg shadow-lift">
        <div className="flex items-center justify-between gap-3 border-b border-line bg-surface px-4 py-3">
          <h2 id={`${uid}-t`} className="text-xl font-bold">Conferir dados</h2>
          <Button variant="secondary" className="min-h-11 px-4 py-2" onClick={onClose}>Fechar</Button>
        </div>

        <form
          id={`${uid}-f`}
          className="flex-1 space-y-5 overflow-y-auto p-4"
          onSubmit={(e) => {
            e.preventDefault()
            if (valid) save.mutate()
          }}
        >
          <p className="text-muted">Veja o que a IA leu. Se algo estiver diferente da fatura, corrija. Refazemos a conta na hora.</p>

          <div className="space-y-1">
            <label htmlFor={`${uid}-op`} className="block font-semibold">Operadora</label>
            <input id={`${uid}-op`} className={field} value={bill.provider_name} onChange={(e) => setBill({ ...bill, provider_name: e.target.value })} />
          </div>
          <div className="space-y-1">
            <label htmlFor={`${uid}-mes`} className="block font-semibold">Mês da fatura</label>
            <input id={`${uid}-mes`} type="month" className={field} value={bill.reference_month} onChange={(e) => e.target.value && setBill({ ...bill, reference_month: e.target.value })} />
          </div>
          <div className="space-y-1">
            <label htmlFor={`${uid}-tot`} className="block font-semibold">Total da fatura</label>
            <MoneyInput id={`${uid}-tot`} value={bill.total_cents} onChange={(c) => setBill({ ...bill, total_cents: c ?? 0 })} />
          </div>

          {bill.energy && (
            <fieldset className="space-y-3 rounded-card border border-line p-4">
              <legend className="px-1 font-semibold">Energia</legend>
              <div className="space-y-1">
                <label htmlFor={`${uid}-kwh`} className="block font-semibold">Consumo (kWh)</label>
                <input
                  id={`${uid}-kwh`}
                  type="text"
                  inputMode="numeric"
                  className={field}
                  value={bill.energy.consumption_kwh}
                  onChange={(e) => {
                    const n = Number(e.target.value.replace(/\D/g, '').slice(0, 6))
                    setBill((b) => (b.energy ? { ...b, energy: { ...b.energy, consumption_kwh: n } } : b))
                  }}
                />
              </div>
              <div className="space-y-1">
                <label htmlFor={`${uid}-leit`} className="block font-semibold">Tipo de leitura</label>
                <select
                  id={`${uid}-leit`}
                  className={field}
                  value={bill.energy.reading_type}
                  onChange={(e) => {
                    const reading_type = e.target.value as ReadingType
                    setBill((b) => (b.energy ? { ...b, energy: { ...b.energy, reading_type } } : b))
                  }}
                >
                  {READINGS.map((r) => <option key={r.value} value={r.value}>{r.label}</option>)}
                </select>
              </div>
            </fieldset>
          )}

          <fieldset className="space-y-3">
            <legend className="font-semibold">Itens da fatura</legend>
            {bill.items.map((it) => (
              <div key={it.id} className="space-y-2 rounded-card border border-line bg-surface p-3">
                <label htmlFor={`${uid}-d-${it.id}`} className="block text-sm font-semibold text-muted">Descrição</label>
                <input id={`${uid}-d-${it.id}`} className={field} value={it.description} onChange={(e) => setItem(it.id, { description: e.target.value })} />
                <label htmlFor={`${uid}-v-${it.id}`} className="block text-sm font-semibold text-muted">Valor</label>
                <MoneyInput id={`${uid}-v-${it.id}`} value={it.amount_cents} onChange={(c) => setItem(it.id, { amount_cents: c ?? 0 })} />
                <p className="font-mono text-xs break-words text-muted">Na fatura: {it.original_text}</p>
              </div>
            ))}
          </fieldset>

          <div aria-live="polite">
            {diverges && (
              <p role="alert" className="rounded-control border border-warning-line bg-warning-soft p-3 font-medium text-warning">
                <span aria-hidden="true">⚠ </span>A soma dos itens não bate com o total. Confira os valores
                <span className="block text-sm font-normal">Soma: {formatBRL(bill.items.reduce((a, i) => a + i.amount_cents, 0))} · Total: {formatBRL(bill.total_cents)}</span>
              </p>
            )}
          </div>
        </form>

        <div className="space-y-2 border-t border-line bg-surface p-4">
          {save.isError && <p role="alert" className="font-medium text-danger">{save.error.message}</p>}
          <div className="flex gap-3">
            <Button type="submit" form={`${uid}-f`} className="flex-1" disabled={!valid || save.isPending}>{save.isPending ? 'Salvando…' : 'Salvar'}</Button>
            <Button variant="secondary" onClick={onClose}>Cancelar</Button>
          </div>
        </div>
      </div>
    </div>
  )
}

export function CorrectionDrawer({ analysis }: { analysis: Analysis }) {
  const [open, setOpen] = useState(false)
  const trigger = useRef<HTMLButtonElement>(null)
  const close = () => {
    setOpen(false)
    trigger.current?.focus()
  }
  return (
    <>
      <button ref={trigger} type="button" onClick={() => setOpen(true)} className="inline-flex min-h-11 items-center text-sm font-semibold text-accent-strong underline">
        Os dados estão errados? Corrigir
      </button>
      {open && <Drawer analysis={analysis} onClose={close} />}
    </>
  )
}
