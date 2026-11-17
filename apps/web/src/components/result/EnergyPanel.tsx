import { useId } from 'react'
import type { EnergyData } from '../../types/EnergyData'
import type { TariffFlag } from '../../types/TariffFlag'
import { consumptionSummary, monthName, monthShort } from '../../lib/derive'
import { Card, Icon } from '../ui'

const FLAGS: Record<TariffFlag, string> = {
  verde: 'Bandeira verde',
  amarela: 'Bandeira amarela',
  vermelha_1: 'Bandeira vermelha 1',
  vermelha_2: 'Bandeira vermelha 2',
  escassez: 'Escassez hídrica',
}
const READING: Record<EnergyData['reading_type'], string> = {
  real: 'Leitura real',
  estimada: 'Leitura estimada',
  media: 'Cobrado pela média',
}

function Chart({ energy, referenceMonth }: { energy: EnergyData; referenceMonth: string }) {
  const titleId = useId()
  const { text, avg } = consumptionSummary(energy, referenceMonth)
  const points = [...energy.history.filter((p) => p.month !== referenceMonth), { month: referenceMonth, kwh: energy.consumption_kwh }].slice(-12)
  const W = 320
  const H = 176
  const top = 24
  const bottom = 28
  const max = Math.max(...points.map((p) => p.kwh), avg, 1)
  const slot = W / points.length
  const bw = slot * 0.62
  const y = (v: number) => top + (H - top - bottom) * (1 - v / max)

  return (
    <figure className="mt-4 max-w-md">
      <svg viewBox={`0 0 ${W} ${H}`} role="img" aria-label={text} className="h-auto w-full">
        <title id={titleId}>{text}</title>
        {points.map((p, i) => {
          const cur = p.month === referenceMonth
          const x = i * slot + (slot - bw) / 2
          return (
            <g key={p.month} aria-hidden="true">
              <rect x={x} y={y(p.kwh)} width={bw} height={H - bottom - y(p.kwh)} rx="2" className={cur ? 'fill-highlight stroke-ink' : 'fill-accent'} strokeWidth={cur ? 2 : 0} />
              {cur && <text x={x + bw / 2} y={y(p.kwh) - 5} textAnchor="middle" fontSize="14" fontWeight="700" className="fill-ink">{p.kwh}</text>}
              <text x={x + bw / 2} y={H - 8} textAnchor="middle" fontSize="14" fontWeight={cur ? '700' : '400'} className={cur ? 'fill-ink' : 'fill-muted'}>{monthShort(p.month)}</text>
            </g>
          )
        })}
        {avg > 0 && (
          <g aria-hidden="true">
            <line x1="0" x2={W} y1={y(avg)} y2={y(avg)} strokeWidth="1.5" strokeDasharray="4 3" className="stroke-ink" />
            <text x="2" y={y(avg) - 4} fontSize="14" strokeWidth="4" strokeLinejoin="round" paintOrder="stroke" className="fill-ink stroke-surface">média {avg}</text>
          </g>
        )}
      </svg>
      <figcaption className="mt-2 text-caption text-muted">{text}. A barra destacada é o mês desta fatura; a linha tracejada é a média.</figcaption>
      <table className="sr-only">
        <caption>Consumo mensal em kWh</caption>
        <thead><tr><th>Mês</th><th>kWh</th></tr></thead>
        <tbody>
          {points.map((p) => <tr key={p.month}><td>{monthName(p.month)}</td><td>{p.kwh}</td></tr>)}
        </tbody>
      </table>
    </figure>
  )
}

export function EnergyPanel({ energy, referenceMonth }: { energy: EnergyData; referenceMonth: string }) {
  const estimated = energy.reading_type !== 'real'
  return (
    <section aria-labelledby="energia-t" className="space-y-4">
      <h2 id="energia-t" className="text-h2">Seu consumo de luz</h2>
      <Card>
        <ul className="flex flex-wrap items-center gap-2">
          <li
            className={`inline-flex items-center gap-1.5 rounded-control border px-2.5 py-1 text-caption font-semibold ${
              estimated ? 'border-warning bg-warning-soft text-warning' : 'border-ok bg-ok-soft text-ok'
            }`}
          >
            <Icon name={estimated ? 'warning' : 'ok'} size={16} />
            {READING[energy.reading_type]}
          </li>
          {energy.tariff_flag && (
            <li className="rounded-control border border-line bg-bg px-2.5 py-1 text-caption font-semibold">{FLAGS[energy.tariff_flag]}</li>
          )}
          <li className="text-caption text-muted">{energy.consumption_kwh} kWh no mês</li>
        </ul>
        {energy.history.length > 0 && <Chart energy={energy} referenceMonth={referenceMonth} />}
      </Card>
    </section>
  )
}
