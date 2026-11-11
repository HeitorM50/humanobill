# 04 — Arquitetura

## Visão geral

```
                ┌───────────────────────────────────────────┐
  Navegador ──▶ │ apps/web  (Vercel)                        │
                │ Vite + React + TS · Tailwind · shadcn     │
                │ TanStack Query · tipos gerados (ts-rs)    │
                └──────────────────┬────────────────────────┘
                                   │ HTTPS / JSON + multipart
                ┌──────────────────▼────────────────────────┐
                │ crates/api  (Render, Docker)              │
                │ Axum · CORS · limite de upload · rate limit│
                │   │                                       │
                │   ├─▶ crates/extract ──▶ Gemini API       │
                │   │     PDF/imagem → JSON (schema)        │
                │   │     + explicações em linguagem simples│
                │   │                                       │
                │   ├─▶ crates/rules   (puro, sem I/O)      │
                │   │     ExtractedBill + Answers → Findings│
                │   │                                       │
                │   └─▶ sqlx ──▶ Supabase Postgres          │
                │         analyses (anonimizadas, 30 dias)  │
                └───────────────────────────────────────────┘
        crates/core: tipos compartilhados (Bill, Item, Finding…) + export ts-rs
```

## Monorepo

```
humanobill/
├── apps/web/                 # front React
├── crates/
│   ├── core/                 # tipos de domínio, serde + ts-rs
│   ├── rules/                # motor de regras (puro, 100% testado)
│   ├── extract/              # cliente Gemini, prompts, validação do JSON
│   └── api/                  # binário Axum (rotas, DB, config)
├── fixtures/
│   ├── bills/                # faturas sintéticas (PDF/PNG)
│   ├── extracted/            # JSON esperado da extração
│   ├── findings/             # achados esperados (golden)
│   └── tariffs/              # bandeiras e tarifas fictícias
├── migrations/               # SQL do Supabase (sqlx)
├── .github/workflows/        # CI (cargo test, clippy, build web) + keep-alive cron
├── docs/
├── Dockerfile                # build da API para o Render
├── LICENSE                   # MIT
└── README.md
```

**Por que separar `rules` e `extract`:** a IA é a parte não determinística e cara. O motor é a parte que dá para provar. Separados, o motor é testado sem rede, e os golden tests rodam no CI de graça.

## Fluxo de uma análise

```
Usuário        Web                 API                  Gemini        Postgres
  │ envia PDF   │                   │                     │              │
  │────────────▶│ POST /analyses    │                     │              │
  │             │ (multipart)       │                     │              │
  │             │──────────────────▶│ valida tipo/tamanho │              │
  │             │                   │ (arquivo só em RAM) │              │
  │             │                   │── generateContent ─▶│              │
  │             │                   │   PDF + prompt +    │              │
  │             │                   │   responseSchema    │              │
  │             │                   │◀── JSON ────────────│              │
  │             │                   │ valida (serde+checks)              │
  │             │                   │ rules::evaluate()   │              │
  │             │                   │── INSERT analysis ─────────────────▶│
  │             │                   │ descarta o arquivo  │              │
  │             │◀── 201 {id, bill, findings, questions}  │              │
  │ responde    │ PATCH /analyses/:id (respostas/correções)              │
  │ perguntas   │──────────────────▶│ rules::evaluate() de novo (sem IA)  │
  │             │◀── findings atualizados                 │              │
```

Pontos-chave:
- **A IA é chamada uma única vez por análise.** Correções e respostas reavaliam só o motor. Isso economiza o free tier e deixa a resposta instantânea.
- **Uma única chamada ao Gemini faz extração + explicações** (o campo `plain_explanation` em cada item), para economizar quota.
- O modo demo usa os **JSON pré-extraídos de `fixtures/extracted/`**. Ele não chama a IA, então é instantâneo, não gasta quota e funciona mesmo se o Gemini cair. Há também um botão "analisar com IA de verdade" para mostrar a extração ao vivo.

## API

| Método | Rota | Descrição |
|---|---|---|
| `GET` | `/health` | `{status:"ok", version}`. Usado pelo keep-alive e pelo warm-up |
| `POST` | `/analyses` | multipart `file` + `consent=true`. Retorna `201 Analysis` |
| `POST` | `/analyses/demo/:sample` | cria uma análise a partir de uma fixture (`telecom-1`, `energia-1`…) |
| `GET` | `/analyses/:id` | retorna `Analysis`, ou `410 Gone` se expirou |
| `PATCH` | `/analyses/:id` | corpo `{answers?, corrections?, paid?}`. Reavalia e retorna `Analysis` |
| `POST` | `/analyses/:id/dispute` | corpo `{finding_ids, protocol?}`. Retorna `{text, steps[]}` (RF11/RF12) |
| `POST` | `/analyses/:id/call-script` | corpo `{finding_ids}`. Retorna `{steps[]}`, o roteiro de ligação (RF17) |
| `POST` | `/comparisons` | corpo `{analysis_ids: [a, b]}`. Retorna `{added[], removed[], changed[], total_delta}` (RF16) |
| `POST` | `/reminders` | corpo `{analysis_id, email, consent:true}`. Retorna `{due_at}` (RF18) |
| `GET` | `/reminders/:token/cancel` | apaga o lembrete e o e-mail (RF19) |
| `POST` | `/internal/reminders/dispatch` | header `X-Cron-Secret`. Envia os lembretes vencidos via Resend e apaga os e-mails |

Erros vêm em `{error: {code, message}}`, com os códigos `invalid_file`, `too_large`, `consent_required`, `extraction_failed`, `rate_limited`, `not_found`, `expired`.

## Schema de extração (resumo)

```text
ExtractedBill {
  kind: "telecom" | "energy",
  provider_name: String,            // "Vivaz Telecom"
  reference_month: "YYYY-MM",
  due_date: Date, issue_date: Option<Date>,
  total_cents: i64,
  plan_name: Option<String>,        // telecom
  items: [ BillItem {
      id, description, amount_cents,
      category: "plano" | "servico_adicional" | "encargo_atraso" | "imposto"
              | "energia" | "bandeira" | "iluminacao_publica" | "outros",
      period: Option<{start,end}>,
      plain_explanation: String     // "Isso é ..."
  } ],
  late_fees: Option<{ base_cents, fine_cents, interest_cents, days_late }>,
  energy: Option<{
      consumption_kwh, reading_type: "real" | "estimada" | "media",
      reading_prev_date, reading_curr_date,
      tariff_flag: "verde" | "amarela" | "vermelha_1" | "vermelha_2" | "escassez",
      history: [{ month, kwh }]     // os 12 meses impressos na fatura
  }>,
  extraction_confidence: 0..1
}
```
**Não existem** campos de nome, CPF/CNPJ do titular, endereço, nº de instalação nem nº de linha. O prompt também manda ignorar esses dados.

**Validação depois da IA:** desserialização estrita com `serde`, soma dos itens vs total, datas coerentes e consumo ≥ 0. Se falhar, uma nova tentativa com um prompt de correção. Se falhar de novo, retorna `extraction_failed` com a sugestão de usar uma foto melhor.

## Modelo de dados (Supabase Postgres)

```sql
create table analyses (
  id            uuid primary key default gen_random_uuid(),
  created_at    timestamptz not null default now(),
  expires_at    timestamptz not null default now() + interval '30 days',
  source        text not null check (source in ('upload','demo')),
  kind          text not null check (kind in ('telecom','energy')),
  bill          jsonb not null,      -- ExtractedBill (já anonimizado por schema)
  answers       jsonb not null default '{}',
  paid          boolean,
  findings      jsonb not null,
  recoverable_cents bigint not null default 0,
  rules_version text not null
);
create index on analyses (expires_at);
```
- **Expiração:** `pg_cron` diário (`delete from analyses where expires_at < now()`) e, como garantia, o `GET` retorna 410 se o registro estiver expirado.
- **Acesso:** só a API acessa o banco (connection string no servidor). O front **nunca** fala direto com o Supabase. A RLS fica ligada, sem policies públicas.
- **Storage:** não é usado no MVP, porque o arquivo não é persistido.

```sql
create table reminders (
  id          uuid primary key default gen_random_uuid(),
  analysis_id uuid not null references analyses(id) on delete cascade,
  email       text not null,                 -- apagado junto com a linha após o envio
  due_at      timestamptz not null,          -- created_at + 30 dias
  cancel_token text not null unique,         -- aleatório, 32 bytes
  created_at  timestamptz not null default now()
);
```
- **Envio diário:** o GitHub Actions cron (1×/dia) chama `POST /internal/reminders/dispatch`. A API envia os e-mails vencidos pelo **Resend** e **deleta** cada linha enviada. O lembrete expira junto com a análise (`on delete cascade`).
- **Comparação (RF16):** é feita sobre duas `analyses` já existentes, sem nova chamada de IA. O diff casa os itens por descrição normalizada.

## Privacidade / LGPD

- **Minimização:** o arquivo vive só em memória durante a requisição. Nome, CPF e endereço não são extraídos.
- **Retenção:** o resultado anonimizado dura 30 dias.
- **Transparência:** uma página `/privacidade` simples explica o que é processado, por quem (Gemini, Google) e por quanto tempo.
- **Free tier do Gemini:** os dados podem ser usados pelo Google para melhorar os modelos. Por isso há aviso + consentimento obrigatório no upload real, e as fixtures são o caminho padrão.
- **Logs:** nunca logar o conteúdo da fatura nem o JSON extraído. Logar só id, duração e códigos de erro.

## Segurança

- Upload: whitelist de MIME (checando os magic bytes, não só a extensão), até 10 MB, até 5 páginas.
- `tower-http`: CORS só para o domínio do front, limite de body e timeout.
- Rate limit por IP (`tower_governor`), por exemplo 10 análises/hora, para proteger a quota do Gemini.
- Segredos (`GEMINI_API_KEY`, `DATABASE_URL`) só em variável de ambiente no Render. `.env` no `.gitignore`.
- Prompt injection: o texto da fatura é dado, nunca instrução. A saída é forçada por `responseSchema` e validada. A IA não tem tools.
- `cargo audit` no CI.

## Observabilidade

- `tracing` com logs JSON no Render: id da análise, latência do Gemini, latência total, regra disparada (só o ID).
- `/health` monitorado pelo UptimeRobot, que também serve de keep-alive e manda alerta por e-mail se cair.

## Testes

| Camada | Como |
|---|---|
| `rules` | Testes unitários por regra (casos positivo, negativo e de borda) |
| Golden | Para cada fixture: `extracted/*.json` → `rules` → compara com `findings/*.json` |
| `extract` | Teste de contrato do schema. Teste live opcional (`--ignored`) contra o Gemini |
| `api` | Testes de rota com `axum::test` e banco em container (ou mock de repositório) |
| Web | Smoke com Playwright no fluxo demo (stretch) |
