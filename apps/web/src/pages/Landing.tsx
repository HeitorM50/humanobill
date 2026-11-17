import { useState } from 'react'
import { useMutation } from '@tanstack/react-query'
import { useNavigate } from 'react-router'
import penUrl from '../assets/caneta-marca-texto.svg'
import disputeUrl from '../assets/passo-conteste.svg'
import understandUrl from '../assets/passo-entenda.svg'
import sendUrl from '../assets/passo-envie.svg'
import { Alert, Button, ButtonLink, Card, Icon, Paper, PaperHeader, PaperLine, PaperTotal } from '../components/ui'
import { createDemoAnalysis, DEMO_GROUPS, DEMO_SAMPLES } from '../lib/api'
import { formatBRL } from '../lib/format'

const STEPS = [
  { t: 'Envie', d: 'Mande o PDF ou uma foto da sua fatura.', img: sendUrl },
  { t: 'Entenda', d: 'Cada item explicado em português simples.', img: understandUrl },
  { t: 'Conteste', d: 'Se algo parecer errado, você leva o texto pronto.', img: disputeUrl },
]

/**
 * Fatura sintética (marca fictícia) que ilustra a ideia do produto: papel, marca-texto e anotação.
 * É decorativa: o título e o subtítulo ao lado já dizem tudo, por isso fica fora da leitura de tela.
 */
function HeroBill() {
  return (
    <div aria-hidden="true" className="relative mb-8 w-full max-w-md md:justify-self-end">
      <Paper className="-rotate-2 text-caption lg:text-body">
        <PaperHeader>
          <span>Vivaz Telecom</span>
          <span className="font-normal text-muted">10/2026</span>
        </PaperHeader>
        <ul>
          <PaperLine label="PLANO VZ CONTROLE 20GB" value="99,90" />
          <PaperLine label="CLUBE VZ MUS" value="12,90" highlight>
            Assinatura de música. Você pediu isso?
          </PaperLine>
          <PaperLine label="SEG TELA" value="9,90" highlight>
            Seguro da tela, cobrado à parte.
          </PaperLine>
        </ul>
        {/* Espaço à direita para a caneta não cobrir o valor. */}
        <PaperTotal className="pr-12">
          <span>Total</span>
          <span>{formatBRL(12270)}</span>
        </PaperTotal>
      </Paper>
      <img src={penUrl} alt="" aria-hidden="true" width={60} height={110} className="absolute -right-4 -bottom-12 h-auto w-18" />
    </div>
  )
}

export default function Landing() {
  const navigate = useNavigate()
  const [choosing, setChoosing] = useState(false)
  const demo = useMutation({
    mutationFn: createDemoAnalysis,
    onSuccess: (a, sample) => navigate(`/r/${a.id}?exemplo=${sample}`),
  })

  return (
    <>
      <section aria-labelledby="abertura-t" className="bg-accent text-on-accent">
        <div className="mx-auto grid max-w-page gap-8 px-5 py-8 md:grid-cols-[1.1fr_0.9fr] md:items-center md:gap-6 md:px-10 md:py-14">
          <div className="flex flex-col gap-5">
            <h1 id="abertura-t" className="text-display">Sua conta, em português claro.</h1>
            <p className="max-w-md text-h4 leading-normal">
              Descubra em segundos se você está pagando o que não deve, e conteste com o texto pronto.
            </p>
            <div className="flex flex-col gap-3 sm:flex-row sm:flex-wrap">
              <Button
                variant="tertiary"
                className="foco-sobre-azul"
                onClick={() => setChoosing((v) => !v)}
                aria-expanded={choosing}
                aria-controls="exemplos"
              >
                Testar com fatura de exemplo
              </Button>
              <ButtonLink to="/analisar" variant="onDark">Enviar minha fatura</ButtonLink>
            </div>
            <p className="flex items-center gap-2 text-caption">
              <Icon name="lock" size={16} />
              Seus dados pessoais não são guardados.
            </p>
          </div>
          <HeroBill />
        </div>
      </section>

      {choosing && (
        <section id="exemplos" aria-labelledby="exemplos-t" className="mx-auto max-w-page px-5 pt-8 md:px-10 md:pt-14">
          <Card>
            <h2 id="exemplos-t" className="text-h3">Escolha um exemplo</h2>
            <div className="mt-4 grid gap-6 md:grid-cols-2">
              {DEMO_GROUPS.map((g, gi) => (
                <div key={g} role="group" aria-labelledby={`grupo-${gi}`}>
                  <h3 id={`grupo-${gi}`} className="font-sans text-h4 tracking-normal text-muted">{g}</h3>
                  <ul className="mt-3 space-y-3">
                    {DEMO_SAMPLES.filter((s) => s.group === g).map((s) => (
                      <li key={s.id}>
                        <Button variant="secondary" className="w-full" disabled={demo.isPending} onClick={() => demo.mutate(s.id)}>
                          <span className="min-w-0 flex-1 text-left break-words">{s.title}</span>
                          <span className="text-caption font-normal text-muted">{s.kind}</span>
                        </Button>
                      </li>
                    ))}
                  </ul>
                </div>
              ))}
            </div>
            {demo.isPending && <p role="status" className="mt-4 text-muted">Abrindo o exemplo…</p>}
            {demo.isError && (
              <Alert tone="danger" role="alert" className="mt-4">
                <p>{demo.error.message}</p>
              </Alert>
            )}
          </Card>
        </section>
      )}

      <section aria-labelledby="como-funciona" className="mx-auto max-w-page px-5 py-8 md:px-10 md:py-14">
        {/* tabIndex -1: o link "Como funciona" da navegação traz o foco para cá. */}
        <h2 id="como-funciona" tabIndex={-1} className="text-h2">Como funciona</h2>
        <ol className="mt-6 grid divide-y divide-line border-y border-line md:grid-cols-3 md:divide-x md:divide-y-0">
          {STEPS.map((s, i) => (
            <li key={s.t} className="grid grid-cols-[auto_1fr] items-center gap-x-4 gap-y-1 py-5 md:px-6 md:first:pl-0 md:last:pr-0">
              <img src={s.img} alt="" aria-hidden="true" width={48} height={48} className="row-span-2 size-12" />
              <h3 className="text-h3">{i + 1}. {s.t}</h3>
              <p className="text-muted">{s.d}</p>
            </li>
          ))}
        </ol>
      </section>
    </>
  )
}
