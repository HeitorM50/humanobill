import { useState, type FormEvent } from 'react'
import { useMutation } from '@tanstack/react-query'
import { Alert, Button, Card, Field } from '../ui'
import { ApiError, createReminder } from '../../lib/api'

// Mesma ideia da API: algo@dominio.tld, sem espaços. A API valida de novo.
const EMAIL_RE = /^[^\s@]+@[^\s@.]+(\.[^\s@.]+)*\.[^\s@.]{2,}$/
const EMAIL_ERROR = 'Informe um e-mail válido'

/** ISO -> "10/12/2026", no horário de Brasília. */
function formatDueDate(iso: string): string {
  return new Intl.DateTimeFormat('pt-BR', { day: '2-digit', month: '2-digit', year: 'numeric', timeZone: 'America/Sao_Paulo' }).format(new Date(iso))
}

/**
 * RF18: lembrete opcional por e-mail no fim do prazo da empresa. O consentimento é próprio,
 * separado do envio da fatura. O e-mail nunca fica guardado no navegador.
 */
export function ReminderForm({ analysisId, showHeading = true }: { analysisId: string; showHeading?: boolean }) {
  const [email, setEmail] = useState('')
  const [consent, setConsent] = useState(false)
  const [invalid, setInvalid] = useState(false)

  const mutation = useMutation({
    mutationFn: () => createReminder({ analysis_id: analysisId, email: email.trim(), consent }),
  })

  const submit = (e: FormEvent) => {
    e.preventDefault()
    if (!consent || mutation.isPending) return
    if (!EMAIL_RE.test(email.trim())) {
      setInvalid(true)
      return
    }
    setInvalid(false)
    mutation.mutate()
  }

  // A API também recusa e-mail inválido: mostra a mesma frase no campo.
  const apiInvalid = mutation.error instanceof ApiError && mutation.error.code === 'invalid_email'
  const fieldError = invalid || apiInvalid ? EMAIL_ERROR : undefined

  return (
    <section
      {...(showHeading ? { 'aria-labelledby': 'lembrete-t' } : { 'aria-label': 'Lembrete por e-mail' })}
      className="space-y-4"
    >
      <div className="max-w-text">
        {/* No passo a passo o título da tela já diz isso: aqui seria repetido. */}
        {showHeading && <h2 id="lembrete-t" className="text-h2">Lembrete por e-mail</h2>}
        <p className="text-muted">Opcional. A gente avisa quando acabar o prazo de 30 dias da empresa para responder.</p>
      </div>
      <Card className="max-w-text">
        {mutation.isSuccess ? (
          <Alert tone="ok" role="status" title={`Vamos te avisar em ${formatDueDate(mutation.data.due_at)}`}>
            <p>Mandamos um e-mail com o link para cancelar. Depois do aviso, seu e-mail é apagado.</p>
          </Alert>
        ) : (
          <form onSubmit={submit} noValidate className="space-y-4">
            <Field label="Seu e-mail" error={fieldError}>
              {(c) => (
                <input
                  {...c}
                  type="email"
                  inputMode="email"
                  autoComplete="email"
                  maxLength={254}
                  value={email}
                  onChange={(e) => {
                    setEmail(e.target.value)
                    setInvalid(false)
                  }}
                />
              )}
            </Field>
            <label className="flex min-h-11 cursor-pointer items-start gap-3 rounded-control border border-line bg-bg px-3 py-2 has-checked:border-accent has-checked:bg-accent-soft">
              <input
                type="checkbox"
                checked={consent}
                onChange={(e) => setConsent(e.target.checked)}
                className="mt-1 size-5.5 shrink-0 accent-accent"
              />
              <span>
                Aceito receber um único e-mail com o aviso. Este consentimento é só do lembrete, separado do envio da fatura.
              </span>
            </label>
            <p className="text-caption text-muted">Seu e-mail é apagado depois do aviso, ou na hora se você cancelar.</p>
            {mutation.isError && !apiInvalid && (
              <Alert tone="danger" role="alert">
                <p>{mutation.error.message}</p>
              </Alert>
            )}
            <Button type="submit" disabled={!consent} loading={mutation.isPending} loadingLabel="Agendando">
              Me lembrar
            </Button>
          </form>
        )}
      </Card>
    </section>
  )
}
