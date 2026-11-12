import { useState } from 'react'
import { useMutation } from '@tanstack/react-query'
import { useNavigate } from 'react-router'
import { Button, ButtonLink, Card } from '../components/ui'
import { createDemoAnalysis, DEMO_GROUPS, DEMO_SAMPLES } from '../lib/api'

const STEPS = [
  { t: 'Envie', d: 'Mande o PDF ou uma foto da sua fatura.' },
  { t: 'Entenda', d: 'Cada item explicado em português simples.' },
  { t: 'Conteste', d: 'Se algo parecer errado, você leva o texto pronto.' },
]

export default function Landing() {
  const navigate = useNavigate()
  const [choosing, setChoosing] = useState(false)
  const demo = useMutation({
    mutationFn: createDemoAnalysis,
    onSuccess: (a, sample) => navigate(`/r/${a.id}?exemplo=${sample}`),
  })

  return (
    <div className="space-y-12">
      <section className="space-y-5">
        <h1 className="text-4xl font-bold tracking-tight text-balance sm:text-5xl">Sua conta, em português humano.</h1>
        <p className="text-lg text-slate-700">
          Descubra em segundos se você está pagando o que não deve, e conteste com o texto pronto.
        </p>
        <div className="flex flex-col gap-3 sm:flex-row">
          <Button onClick={() => setChoosing((v) => !v)} aria-expanded={choosing} aria-controls="exemplos">
            Testar com fatura de exemplo
          </Button>
          <ButtonLink to="/analisar" variant="secondary">Enviar minha fatura</ButtonLink>
        </div>
        <p className="text-sm text-slate-600">🔒 Seus dados pessoais não são guardados.</p>
      </section>

      {choosing && (
        <section id="exemplos" aria-labelledby="exemplos-t">
          <Card>
            <h2 id="exemplos-t" className="text-lg font-semibold">Escolha um exemplo</h2>
            {DEMO_GROUPS.map((g, gi) => (
              <div key={g} role="group" aria-labelledby={`grupo-${gi}`} className="mt-4">
                <h3 id={`grupo-${gi}`} className="text-sm font-semibold text-slate-600">{g}</h3>
                <ul className="mt-2 space-y-2">
                  {DEMO_SAMPLES.filter((s) => s.group === g).map((s) => (
                    <li key={s.id}>
                      <Button
                        variant="secondary"
                        className="w-full justify-between text-left"
                        disabled={demo.isPending}
                        onClick={() => demo.mutate(s.id)}
                      >
                        <span>{s.title}</span>
                        <span className="text-sm font-normal text-slate-600">{s.kind}</span>
                      </Button>
                    </li>
                  ))}
                </ul>
              </div>
            ))}
            {demo.isPending && <p role="status" className="mt-3 text-slate-700">Abrindo o exemplo…</p>}
            {demo.isError && <p role="alert" className="mt-3 font-medium text-red-800">{demo.error.message}</p>}
          </Card>
        </section>
      )}

      <section aria-labelledby="como-t">
        <h2 id="como-t" className="text-2xl font-bold">Como funciona</h2>
        <ol className="mt-4 grid gap-3 sm:grid-cols-3">
          {STEPS.map((s, i) => (
            <li key={s.t}>
              <Card className="h-full">
                <span className="grid size-8 place-items-center rounded-full bg-emerald-100 font-bold text-emerald-900" aria-hidden="true">{i + 1}</span>
                <h3 className="mt-2 text-lg font-semibold">{s.t}</h3>
                <p className="text-slate-700">{s.d}</p>
              </Card>
            </li>
          ))}
        </ol>
      </section>
    </div>
  )
}
