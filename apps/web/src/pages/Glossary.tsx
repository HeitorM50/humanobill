import { useMemo } from 'react'
import { useSearchParams } from 'react-router'
import { Button, Card, Field, Page } from '../components/ui'
import { GLOSSARY, searchGlossary } from '../lib/glossary'

const termos = (n: number) => `${n} ${n === 1 ? 'termo' : 'termos'}`

/** Glossário (RF24): os termos que aparecem nas faturas, em português claro. A busca vive na URL (`?q=`). */
export default function Glossary() {
  const [params, setParams] = useSearchParams()
  const q = params.get('q') ?? ''
  const found = useMemo(() => searchGlossary(q), [q])

  const setQuery = (value: string) =>
    // `replace`: cada letra digitada não vira uma entrada no histórico.
    setParams(value ? { q: value } : {}, { replace: true })

  return (
    <Page width="form" className="space-y-6">
      <header className="space-y-3">
        <h1 className="text-h1">Glossário</h1>
        <p className="max-w-text text-muted">Os termos que aparecem na sua fatura, explicados em poucas palavras.</p>
      </header>

      <div className="flex flex-wrap items-end gap-3">
        <Field label="Buscar um termo" className="min-w-0 flex-1 basis-64">
          {(c) => (
            <input
              {...c}
              type="search"
              value={q}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Por exemplo: bandeira"
              autoComplete="off"
              enterKeyHint="search"
            />
          )}
        </Field>
        {q && (
          <Button variant="secondary" onClick={() => setQuery('')} className="min-h-14!">
            Limpar a busca
          </Button>
        )}
      </div>

      <p role="status" className="text-muted">
        {found.length === 0
          ? 'Nenhum termo encontrado. Tente outra palavra ou limpe a busca.'
          : q.trim()
            ? `${termos(found.length)} para a sua busca.`
            : `${termos(GLOSSARY.length)} no glossário.`}
      </p>

      <ul className="space-y-4">
        {found.map((t) => (
          <li key={t.id}>
            {/* `tabIndex=-1`: o link "Ver no glossário" leva o foco até aqui (ver Layout). */}
            <Card id={t.id} tabIndex={-1} className="scroll-mt-6 space-y-2 target:border-accent target:bg-accent-soft">
              <h2 className="text-h3">{t.term}</h2>
              <p className="font-semibold">{t.short}</p>
              <p className="max-w-text">{t.long}</p>
            </Card>
          </li>
        ))}
      </ul>
    </Page>
  )
}
