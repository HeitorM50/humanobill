import { useEffect, useRef, useState, type FormEvent, type ReactNode } from 'react'
import { Link } from 'react-router'
import { Alert, Button, Card, Field, Page } from '../components/ui'
import { DEMO_SAMPLES, sendWhatsappSim, type BotMessage, type WhatsappSimInput } from '../lib/api'

/** Mesmo limite do servidor para o texto do simulador. */
const MAX_TEXT = 300
/** Pausa mínima do "digitando…", para a resposta não surgir de repente. */
const TYPING_MS = 450

type Line = {
  id: number
  from: 'user' | 'bot'
  text: string
  /** Presente quando a mensagem do bot traz o link de um resultado. */
  resultId: string | null
  time: string
}

const horario = () => new Date().toLocaleTimeString('pt-BR', { hour: '2-digit', minute: '2-digit' })

const reducedMotion = () => window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false
const pausa = (ms: number) => new Promise<void>((r) => setTimeout(r, ms))

/** Troca o link do resultado, escrito por extenso na mensagem, por um link que abre `/r/<id>` aqui mesmo. */
function renderText(text: string, resultId: string | null): ReactNode {
  if (!resultId) return text
  const m = new RegExp(`https?://\\S*/r/${resultId}`).exec(text)
  if (!m) return text
  return (
    <>
      {text.slice(0, m.index)}
      <Link
        to={`/r/${resultId}`}
        className="inline-block min-h-11 break-all py-2.5 font-semibold text-accent underline underline-offset-4"
      >
        {m[0]}
      </Link>
      {text.slice(m.index + m[0].length)}
    </>
  )
}

function Bubble({ line }: { line: Line }) {
  const mine = line.from === 'user'
  return (
    <li className={`flex ${mine ? 'justify-end' : 'justify-start'}`}>
      <div
        className={`max-w-[88%] rounded-card px-4 py-2.5 sm:max-w-[80%] ${
          mine ? 'bg-accent text-on-accent' : 'border border-line bg-surface text-ink'
        }`}
      >
        <p className="sr-only">{mine ? 'Você disse:' : 'HumanoBill respondeu:'}</p>
        <p className="break-words whitespace-pre-line">{mine ? line.text : renderText(line.text, line.resultId)}</p>
        <p className={`mt-1 text-right text-caption ${mine ? 'text-on-accent' : 'text-muted'}`}>
          <time>{line.time}</time>
        </p>
      </div>
    </li>
  )
}

export default function WhatsAppSim() {
  const [lines, setLines] = useState<Line[]>([])
  const [typing, setTyping] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [draft, setDraft] = useState('')
  const nextId = useRef(1)
  const end = useRef<HTMLLIElement>(null)

  useEffect(() => {
    end.current?.scrollIntoView?.({ block: 'nearest' })
  }, [lines, typing])

  const add = (from: Line['from'], text: string, resultId: string | null = null) =>
    setLines((l) => [...l, { id: nextId.current++, from, text, resultId, time: horario() }])

  async function send(shown: string, input: WhatsappSimInput) {
    if (typing) return
    setError(null)
    add('user', shown)
    setTyping(true)
    try {
      const [reply] = await Promise.all([sendWhatsappSim(input), reducedMotion() ? undefined : pausa(TYPING_MS)])
      reply.mensagens.forEach((m: BotMessage) => add('bot', m.texto, m.resultado_id))
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Algo deu errado. Tente de novo.')
    } finally {
      setTyping(false)
    }
  }

  function onSubmit(ev: FormEvent) {
    ev.preventDefault()
    const texto = draft.trim()
    if (!texto || typing) return
    setDraft('')
    void send(texto, { texto })
  }

  return (
    <Page width="form" className="space-y-6">
      <header className="space-y-3">
        <h1 className="text-h1">Simulador de conversa</h1>
        <p className="max-w-text text-muted">
          Veja como o HumanoBill responde numa conversa de mensagens: o total da conta, o que parece cobrado a mais e o link
          com tudo explicado.
        </p>
        <Alert tone="info">
          <p>Simulação: o bot real precisa de um número aprovado na Meta. Aqui só funcionam texto e faturas de exemplo.</p>
        </Alert>
      </header>

      <section aria-labelledby="sim-conversa" className="space-y-3">
        <h2 id="sim-conversa" className="sr-only">
          Conversa
        </h2>
        <div className="overflow-hidden rounded-card border border-line bg-accent-soft">
          <div className="border-b border-line bg-surface px-4 py-3">
            <p className="font-bold">HumanoBill</p>
            <p className="text-caption text-muted">{typing ? 'digitando…' : 'Simulador de conversa'}</p>
          </div>
          {/* O log é anunciado aos poucos (aria-live) e rola pelo teclado (tabIndex). */}
          <ol
            role="log"
            aria-live="polite"
            aria-relevant="additions"
            aria-label="Mensagens da conversa"
            tabIndex={0}
            className="flex h-[26rem] max-h-[60dvh] flex-col gap-3 overflow-y-auto p-4"
          >
            {lines.length === 0 && (
              <li className="m-auto max-w-xs text-center text-muted">
                Nenhuma mensagem ainda. Escolha um exemplo ou escreva MENU.
              </li>
            )}
            {lines.map((l) => (
              <Bubble key={l.id} line={l} />
            ))}
            {typing && (
              <li className="flex justify-start">
                <div className="rounded-card border border-line bg-surface px-4 py-2.5 text-muted">
                  <span className="motion-safe:animate-pulse">HumanoBill está digitando…</span>
                </div>
              </li>
            )}
            <li ref={end} aria-hidden="true" className="h-0" />
          </ol>
        </div>
        {error && (
          <Alert tone="danger" role="alert">
            <p>{error}</p>
          </Alert>
        )}
      </section>

      <Card className="space-y-4">
        <div role="group" aria-labelledby="sim-exemplos" className="space-y-3">
          <h2 id="sim-exemplos" className="text-h4">
            Testar com um exemplo
          </h2>
          <ul className="grid gap-2 sm:grid-cols-2">
            {DEMO_SAMPLES.map((s) => (
              <li key={s.id} className="grid">
                <Button
                  variant="secondary"
                  disabled={typing}
                  onClick={() => void send(`Exemplo: ${s.title}`, { exemplo: s.id })}
                >
                  {s.title}
                </Button>
              </li>
            ))}
          </ul>
          <Button variant="ghost" disabled={typing} onClick={() => void send('Menu', { texto: 'menu' })}>
            Ver o menu
          </Button>
        </div>

        <form onSubmit={onSubmit} className="space-y-3 border-t border-line pt-4">
          <Field label="Escreva uma mensagem" hint="Por exemplo: EXEMPLO, EXEMPLO LUZ ou MENU.">
            {(c) => (
              <input
                {...c}
                type="text"
                value={draft}
                maxLength={MAX_TEXT}
                autoComplete="off"
                onChange={(e) => setDraft(e.target.value)}
              />
            )}
          </Field>
          <Button type="submit" disabled={typing || draft.trim() === ''}>
            Enviar
          </Button>
        </form>
      </Card>
    </Page>
  )
}
