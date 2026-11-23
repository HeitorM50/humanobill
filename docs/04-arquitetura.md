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
| `POST` | `/analyses/:id/dispute` | corpo `{finding_ids, protocol?}`. Retorna `{text, steps[], call_script[], …}`: a contestação (RF11/RF12) e o roteiro de ligação (RF17, `call_script[]` com `{title, say}`). `protocol` aceita só dígitos e hífen, até 30 caracteres |
| `POST` | `/comparisons` | corpo `{analysis_ids: [a, b]}` (em qualquer ordem). Só lê, nada é gravado, e vale o mesmo rate limit por IP das demais escritas. Retorna `{kind, provider_name, before_month, after_month, before_total_cents, after_total_cents, total_delta_cents, total_delta_pct, added[], removed[], changed[], unchanged[]}` (RF16) |
| `GET` | `/stats` | painel coletivo anonimizado (#62). Conta só análises enviadas (`source = 'upload'`) e não expiradas, nunca as de demo. Retorna `{min_group, kinds[], rules[]}`: `kinds[]` com `{kind, total, has_hidden_findings}` e `rules[]` com `{rule_id, title, kind, severity, count, percent}`. **K-anonimato:** grupo com menos de 5 análises não aparece (`total` vira `null` e a regra sai de `rules[]`). Sem empresa, sem valor, sem ids. Cache em memória de 5 minutos; `Cache-Control: public, max-age=60` |
| `POST` | `/reminders` | corpo `{analysis_id, email, consent:true}`. Retorna `{due_at}` (RF18) |
| `GET` | `/reminders/:token/cancel` | apaga o lembrete e o e-mail (RF19) |
| `POST` | `/internal/reminders/dispatch` | header `X-Cron-Secret` (comparado em tempo constante; sem segredo configurado a rota responde 503). Envia os lembretes vencidos via Resend e apaga os e-mails. Retorna só contagens: `{sent, failed, dropped}` |
| `GET` | `/whatsapp/webhook` | verificação da Meta (#58): `hub.mode=subscribe`, `hub.verify_token` (comparado em tempo constante com `WHATSAPP_VERIFY_TOKEN`) e `hub.challenge`, que volta como texto puro. Token errado: `403`. **Sem as quatro variáveis `WHATSAPP_*`: `404`** |
| `POST` | `/whatsapp/webhook` | mensagens recebidas, no formato da WhatsApp Cloud API. Confere o header `X-Hub-Signature-256` (HMAC-SHA256 do corpo cru com `WHATSAPP_APP_SECRET`, comparação em tempo constante); sem assinatura ou com assinatura errada: `401` e nada é processado. Lê texto, foto e documento, baixa a mídia pela Graph API e roda o **mesmo pipeline do upload** (`upload::analyze_upload`). Responde `200` na hora e processa em segundo plano; a resposta ao usuário sai pelo endpoint de envio da Graph API. Sem as variáveis: `404` |
| `POST` | `/whatsapp/simulador` | corpo `{texto}` **ou** `{exemplo}` (id de uma fatura de exemplo). Passa pela mesma lógica de conversa do bot, usa só o caminho da demo (**nunca chama a IA**, não aceita arquivo) e devolve `{mensagens: [{texto, resultado_id}]}`. Sempre ligado, com o mesmo rate limit por IP das rotas de escrita. A página é `/simulador-whatsapp` |

Erros vêm em `{error: {code, message}}`, com os códigos `invalid_file`, `too_large`, `consent_required`, `extraction_failed`, `rate_limited`, `not_found`, `expired`, `invalid_protocol` (protocolo fora de dígitos e hífen, ou acima de 30), `same_month` (comparação de faturas do mesmo mês), `different_provider` (comparação de operadoras diferentes), `invalid_signature` (webhook do WhatsApp sem assinatura válida) e `verification_failed` (verificação do webhook recusada).

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
- **Envio diário:** o GitHub Actions cron (1×/dia) chama `POST /internal/reminders/dispatch`. A API envia os e-mails vencidos pelo **Resend** e **deleta** cada linha enviada. O lembrete expira junto com a análise (`on delete cascade`). Recusa **permanente** do Resend (4xx, exceto 408 e 429, como endereço inválido) apaga a linha e o e-mail na hora (`dropped`); erro transitório (rede, 408, 429, 5xx) mantém para a próxima rodada.
- **Data do aviso (RF18):** `due_at = min(hoje + 30 dias, expiração da análise − 1 dia)`. Sem o limite, o `cascade` apagaria o lembrete antes do envio. Uma análise nova avisa em 29 dias.
- **Confirmação:** ao agendar, a API envia um e-mail de confirmação com o link de cancelamento (sem ele o RF19 não teria link). Se esse envio falhar, o agendamento é desfeito e o e-mail não é guardado. O link abre a página `/lembrete/cancelar/:token` do front, e só o JS chama `GET /reminders/:token/cancel`, para o pré-carregamento de links de e-mail não cancelar sozinho.
- **Comparação (RF16):** é feita sobre duas `analyses` já existentes, sem nova chamada de IA. O diff casa os itens por descrição normalizada. A comparação fica em `crates/rules/src/compare.rs` e o roteiro de ligação em `crates/rules/src/dispute.rs`, ambos funções puras, sem IA.

## Privacidade / LGPD

- **Minimização:** o arquivo vive só em memória durante a requisição. Nome, CPF e endereço não são extraídos.
- **Retenção:** o resultado anonimizado dura 30 dias.
- **Transparência:** uma página `/privacidade` simples explica o que é processado, por quem (Gemini, Google) e por quanto tempo.
- **Free tier do Gemini:** os dados podem ser usados pelo Google para melhorar os modelos. Por isso há aviso + consentimento obrigatório no upload real, e as fixtures são o caminho padrão.
- **Estatística coletiva (#62):** `GET /stats` só devolve contagens agregadas de análises enviadas (nunca de demo), e só de grupos com pelo menos 5 casos. Não devolve empresa, valor, texto da fatura nem id de análise. A página `/privacidade` avisa disso.
- **Logs:** nunca logar o conteúdo da fatura nem o JSON extraído. Logar só id, duração e códigos de erro.
- **Bot do WhatsApp (#58):** o número de telefone, o texto das mensagens e o id da mídia nunca vão para log, para o banco nem para erro (os tipos que os carregam têm `Debug` que os esconde). O número só existe em memória durante o atendimento e, para os limites de uso, como hash SHA-256. O arquivo da fatura segue a regra do upload: só em memória. O menu avisa antes do envio que a leitura é feita por IA do Google.

## Segurança

- Upload: whitelist de MIME (checando os magic bytes, não só a extensão), até 10 MB, até 5 páginas.
- `tower-http`: CORS só para o domínio do front, limite de body e timeout.
- Rate limit por IP (`tower_governor`), por exemplo 10 análises/hora, para proteger a quota do Gemini.
- Webhook do WhatsApp (#58): desligado por padrão (404). Ligado, só aceita corpo com assinatura HMAC válida (`X-Hub-Signature-256`), comparada em tempo constante. Não usa rate limit por IP (os IPs são da Meta); no lugar, limita por remetente (5 faturas lidas pela IA e 20 exemplos por hora) e no total (40 faturas por hora), descarta entregas repetidas da mesma mensagem e só baixa mídia por `https`. O simulador usa o limite por IP das rotas de escrita e não aceita arquivo.
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
