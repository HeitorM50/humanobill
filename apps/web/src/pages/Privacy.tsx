import { Page } from '../components/ui'

const SECTIONS = [
  { t: 'O que processamos', d: 'A fatura que você envia, só na memória, durante a análise. Depois disso, guardamos apenas o resultado sem dados pessoais.' },
  { t: 'Quem processa', d: 'A leitura é feita pela IA Google Gemini, no plano gratuito. Nesse plano, o provedor pode usar os dados para melhorar os produtos dele. Por isso, prefira as faturas de exemplo.' },
  { t: 'Por quanto tempo', d: 'O resultado anonimizado fica disponível por 30 dias e depois é apagado.' },
  { t: 'O que nunca guardamos', d: 'Seu nome, CPF, endereço e o arquivo da fatura.' },
]

export default function Privacy() {
  return (
    <Page>
      <article className="max-w-text space-y-8">
        <header className="space-y-2">
          <h1 className="text-h1">Privacidade</h1>
          <p className="text-caption text-muted">Atualizada em <time dateTime="2026-11-11">11/11/2026</time></p>
        </header>
        {SECTIONS.map((s) => (
          <section key={s.t} className="space-y-2">
            <h2 className="text-h3">{s.t}</h2>
            <p>{s.d}</p>
          </section>
        ))}
      </article>
    </Page>
  )
}
