const SECTIONS = [
  { t: 'O que processamos', d: 'A fatura que você envia, só na memória, durante a análise. Depois disso, guardamos apenas o resultado sem dados pessoais.' },
  { t: 'Quem processa', d: 'A leitura é feita pela IA Google Gemini, no plano gratuito. Nesse plano, o provedor pode usar os dados para melhorar os produtos dele. Por isso, prefira as faturas de exemplo.' },
  { t: 'Por quanto tempo', d: 'O resultado anonimizado fica disponível por 30 dias e depois é apagado.' },
  { t: 'O que nunca guardamos', d: 'Seu nome, CPF, endereço e o arquivo da fatura.' },
]

export default function Privacy() {
  return (
    <article className="space-y-6">
      <h1 className="text-3xl font-bold tracking-tight">Privacidade</h1>
      <p className="text-sm text-slate-600">Atualizada em <time dateTime="2026-11-11">11/11/2026</time></p>
      {SECTIONS.map((s) => (
        <section key={s.t}>
          <h2 className="text-xl font-semibold">{s.t}</h2>
          <p className="mt-1 text-slate-700">{s.d}</p>
        </section>
      ))}
    </article>
  )
}
