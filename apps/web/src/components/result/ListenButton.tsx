import { useEffect, useRef, useState } from 'react'
import { buildSpeech, isSpeechSupported } from '../../lib/speech'
import type { Analysis } from '../../types/Analysis'
import { Button } from '../ui'

/** Alto-falante (ouvir) e quadrado (parar). Decorativos: o texto do botão diz a ação. */
function SpeakerIcon({ playing }: { playing: boolean }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={2.2}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      className="size-5 shrink-0"
    >
      {playing ? (
        <rect x="6" y="6" width="12" height="12" rx="1.5" />
      ) : (
        <>
          <path d="M4 9.5h3.5L12 5.5v13l-4.5-4H4z" />
          <path d="M15.5 9a4 4 0 0 1 0 6M18 6.5a7.5 7.5 0 0 1 0 11" />
        </>
      )}
    </svg>
  )
}

/**
 * Botão "Ouvir" / "Parar" (RF21): lê o resultado em pt-BR com a síntese de voz do navegador.
 * Sem `speechSynthesis`, não mostra nada. O texto vem de `buildSpeech`.
 */
export function ListenButton({ analysis }: { analysis: Analysis }) {
  const [speaking, setSpeaking] = useState(false)
  // Cada leitura tem um número. Cancelar faz o navegador avisar as falas antigas com erro: o número deixa ignorar esses avisos.
  const run = useRef(0)
  const speakingRef = useRef(false)

  // Ao sair da tela, a voz não pode continuar falando sozinha.
  useEffect(
    () => () => {
      if (!speakingRef.current) return
      run.current += 1 // invalida a leitura antes de cancelar, para os avisos de erro não mexerem no estado
      speakingRef.current = false
      window.speechSynthesis.cancel()
    },
    [],
  )

  if (!isSpeechSupported()) return null

  const stop = () => {
    run.current += 1
    speakingRef.current = false
    window.speechSynthesis.cancel()
    setSpeaking(false)
  }

  const start = () => {
    run.current += 1
    const id = run.current
    window.speechSynthesis.cancel() // limpa qualquer fala que tenha sobrado
    speakingRef.current = true
    setSpeaking(true)
    const lines = buildSpeech(analysis)
    lines.forEach((text, i) => {
      const utterance = new SpeechSynthesisUtterance(text)
      utterance.lang = 'pt-BR'
      if (i === lines.length - 1) {
        utterance.onend = () => {
          if (run.current !== id) return
          speakingRef.current = false
          setSpeaking(false)
        }
      }
      // Se a voz falhar, para tudo: não deixa o botão dizendo "Parar" sem som.
      utterance.onerror = () => {
        if (run.current === id) stop()
      }
      window.speechSynthesis.speak(utterance)
    })
  }

  return (
    <Button variant="ghost" onClick={speaking ? stop : start}>
      <SpeakerIcon playing={speaking} />
      {speaking ? 'Parar' : 'Ouvir'}
    </Button>
  )
}
