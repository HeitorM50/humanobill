import { useEffect, useId, useRef, useState } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { patchAnalysis } from '../../lib/api'
import { sumDiverges } from '../../lib/derive'
import { formatBRL } from '../../lib/format'
import type { Analysis } from '../../types/Analysis'
import type { ExtractedBill } from '../../types/ExtractedBill'
import type { ReadingType } from '../../types/ReadingType'
import { MoneyInput } from '../MoneyInput'
import { Alert, Button, Field, Icon } from '../ui'

const READINGS: { value: ReadingType; label: string }[] = [
  { value: 'real', label: 'Leitura real' },
  { value: 'estimada', label: 'Leitura estimada' },
  { value: 'media', label: 'Pela média' },
]
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
    <div
      className="fixed inset-0 z-50 flex items-end bg-ink/50 md:items-stretch md:justify-end"
      onMouseDown={(e) => e.target === e.currentTarget && onClose()}
      onKeyDown={onKeyDown}
    >
      <div
        ref={panel}
        role="dialog"
        aria-modal="true"
        aria-labelledby={`${uid}-t`}
        className="gaveta-entra flex max-h-[90dvh] w-full flex-col rounded-t-card bg-surface text-ink shadow-lift md:max-h-none md:w-120 md:rounded-tr-none md:rounded-bl-card"
      >
        <div className="flex items-center justify-between gap-3 border-b border-line px-5 py-3">
          <h2 id={`${uid}-t`} className="text-h3">Conferir dados</h2>
          <Button variant="ghost" onClick={onClose}>
            <Icon name="close" size={16} />
            Fechar
          </Button>
        </div>

        <form
          id={`${uid}-f`}
          className="flex-1 space-y-5 overflow-y-auto p-5"
          onSubmit={(e) => {
            e.preventDefault()
            if (valid) save.mutate()
          }}
        >
          <p className="text-muted">Veja o que a IA leu. Se algo estiver diferente da fatura, corrija. Refazemos a conta na hora.</p>

          <Field label="Operadora">
            {(c) => <input {...c} value={bill.provider_name} onChange={(e) => setBill({ ...bill, provider_name: e.target.value })} />}
          </Field>
          <Field label="Mês da fatura">
            {(c) => (
              <input {...c} type="month" value={bill.reference_month} onChange={(e) => e.target.value && setBill({ ...bill, reference_month: e.target.value })} />
            )}
          </Field>
          <Field label="Total da fatura">
            {(c) => <MoneyInput {...c} value={bill.total_cents} onChange={(v) => setBill({ ...bill, total_cents: v ?? 0 })} />}
          </Field>

          {bill.energy && (
            <fieldset className="space-y-4 rounded-card border border-line p-4">
              <legend className="px-1 font-semibold">Energia</legend>
              <Field label="Consumo (kWh)">
                {(c) => (
                  <input
                    {...c}
                    type="text"
                    inputMode="numeric"
                    value={bill.energy?.consumption_kwh ?? 0}
                    onChange={(e) => {
                      const n = Number(e.target.value.replace(/\D/g, '').slice(0, 6))
                      setBill((b) => (b.energy ? { ...b, energy: { ...b.energy, consumption_kwh: n } } : b))
                    }}
                  />
                )}
              </Field>
              <Field label="Tipo de leitura">
                {(c) => (
                  <select
                    {...c}
                    value={bill.energy?.reading_type}
                    onChange={(e) => {
                      const reading_type = e.target.value as ReadingType
                      setBill((b) => (b.energy ? { ...b, energy: { ...b.energy, reading_type } } : b))
                    }}
                  >
                    {READINGS.map((r) => <option key={r.value} value={r.value}>{r.label}</option>)}
                  </select>
                )}
              </Field>
            </fieldset>
          )}

          <fieldset className="space-y-3">
            <legend className="mb-3 font-semibold">Itens da fatura</legend>
            {bill.items.map((it) => (
              <div key={it.id} className="space-y-3 rounded-card border border-line bg-bg p-4">
                <Field label="Descrição">
                  {(c) => <input {...c} value={it.description} onChange={(e) => setItem(it.id, { description: e.target.value })} />}
                </Field>
                <Field label="Valor">
                  {(c) => <MoneyInput {...c} value={it.amount_cents} onChange={(v) => setItem(it.id, { amount_cents: v ?? 0 })} />}
                </Field>
                <p className="font-mono text-caption break-words text-muted">Na fatura: {it.original_text}</p>
              </div>
            ))}
          </fieldset>

          <div aria-live="polite">
            {diverges && (
              <Alert tone="warning" role="alert">
                <p>
                  A soma dos itens não bate com o total. Confira os valores
                  <span className="block text-caption font-normal">Soma: {formatBRL(bill.items.reduce((a, i) => a + i.amount_cents, 0))} · Total: {formatBRL(bill.total_cents)}</span>
                </p>
              </Alert>
            )}
          </div>
        </form>

        <div className="space-y-3 border-t border-line p-5">
          {save.isError && (
            <Alert tone="danger" role="alert">
              <p>{save.error.message}</p>
            </Alert>
          )}
          <div className="flex justify-end gap-3">
            <Button variant="ghost" onClick={onClose}>Cancelar</Button>
            <Button type="submit" form={`${uid}-f`} disabled={!valid} loading={save.isPending} loadingLabel="Salvando…">Salvar</Button>
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
      <Button ref={trigger} variant="ghost" onClick={() => setOpen(true)}>
        Os dados estão errados? Corrigir
      </Button>
      {open && <Drawer analysis={analysis} onClose={close} />}
    </>
  )
}
