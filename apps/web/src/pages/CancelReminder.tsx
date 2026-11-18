import { useQuery } from '@tanstack/react-query'
import { useParams } from 'react-router'
import { Alert, ButtonLink, Page, Spinner } from '../components/ui'
import { ApiError, cancelReminder } from '../lib/api'

/**
 * RF19: o link do e-mail abre esta página, que cancela o lembrete e apaga o e-mail.
 * Link inexistente e já cancelado mostram a mesma mensagem (não revela se o token existiu).
 */
export default function CancelReminder() {
  const { token = '' } = useParams()
  // Uma única chamada: cancelar de novo daria "já foi cancelado" logo após o sucesso.
  const query = useQuery({
    queryKey: ['cancelar-lembrete', token],
    queryFn: () => cancelReminder(token),
    staleTime: Infinity,
    gcTime: Infinity,
    retry: false,
  })

  return (
    <Page width="form" className="space-y-6">
      <h1 className="text-h2">Cancelar lembrete</h1>
      <div aria-live="polite">
        {query.isPending && (
          <p className="inline-flex items-center gap-2 text-muted">
            <Spinner />
            Cancelando…
          </p>
        )}
        {query.isSuccess && <Alert tone="ok" role="status" title="Lembrete cancelado e e-mail apagado" />}
        {query.isError && (
          <Alert tone={query.error instanceof ApiError && query.error.status === 404 ? 'info' : 'danger'} title={query.error.message} />
        )}
      </div>
      <ButtonLink to="/" variant="secondary">Voltar ao início</ButtonLink>
    </Page>
  )
}
