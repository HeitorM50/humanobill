# 05 — Stack

> As versões exatas são fixadas no dia 1 do roadmap (`Cargo.toml` / `package.json`), sempre usando a versão estável mais recente.

## Front-end — `apps/web`

| Tecnologia | Por quê | Alternativa descartada |
|---|---|---|
| **Vite + React + TypeScript** | É onde o Heitor já é produtivo, e o build é rápido. SPA basta: não precisa de SEO nem de SSR | Next.js: SSR desnecessário, já que o back é Rust |
| **Tailwind CSS** | Velocidade + design system por tokens | CSS modules |
| **shadcn/ui** | Componentes acessíveis e customizáveis, visual "fintech clean" | MUI (tem cara de template) |
| **Motion** (ex-Framer Motion) | Micro-animações nos achados e no contador de valor | — |
| **TanStack Query** | Cache e estados de loading/erro da API | fetch manual |
| **React Router** | Rotas `/`, `/analisar`, `/r/:id`, `/privacidade` | — |
| **react-dropzone** | Upload por arrastar e câmera no celular | input nativo cru |
| **Web Speech API** (`speechSynthesis`, pt-BR) | "Ouvir explicação" (RF21). Nativa do navegador, custo zero | TTS em nuvem (pago) |
| **Recharts** (ou SVG próprio) | Composição da conta (RF23) e histórico de 12 meses | Chart.js |
| **Tipos via ts-rs** | Os tipos TS são gerados das structs Rust, então front e back nunca divergem | Escrever os tipos à mão |

## Back-end — `crates/*`

| Tecnologia | Por quê | Alternativa descartada |
|---|---|---|
| **Rust (edição estável)** | Interesse do Heitor. Tipagem forte para dinheiro e regras. Binário pequeno com cold start de ~1 s | Node/TS |
| **Axum + Tokio** | Framework web padrão do ecossistema, ergonômico | Actix |
| **tower-http / tower_governor** | CORS, limite de body, timeout, rate limit | middleware manual |
| **serde / serde_json** | (De)serialização estrita do JSON da IA | — |
| **reqwest** | Chamada REST ao Gemini. Não há SDK oficial em Rust, e o REST é simples | crates de terceiros sem manutenção |
| **sqlx (Postgres)** | Queries checadas, migrations | Diesel (mais cerimônia) |
| **ts-rs** | Exporta os tipos para o front | specta |
| **thiserror / anyhow** | Erros tipados no domínio, anyhow na borda | — |
| **tracing** | Logs estruturados | log/env_logger |
| **infer** | Detecta o MIME pelos magic bytes | confiar na extensão |

**Dica de produtividade para quem tem Rust intermediário:** manter o `rules` sem `async` e sem lifetimes complicadas (structs donas dos dados, `Vec`, `String`). O async fica só no `api` e no `extract`.

## IA

| | |
|---|---|
| **Provedor** | Google **Gemini API**, free tier (decisão do Heitor) |
| **Modelo** | **`gemini-3.6-flash`** (Flash estável em 11/11/2026; o `gemini-2.5-flash` foi descontinuado e, a partir de 01/12/2026, é roteado para o `gemini-3.5-flash-lite`). Configurável por `GEMINI_MODEL`. Fallback: `gemini-3.5-flash-lite`. As quotas do free tier não aparecem na documentação pública, só no painel do AI Studio ([aistudio.google.com/rate-limit](https://aistudio.google.com/rate-limit)). **Quotas lidas em 19/11/2026 (por modelo, pedidos por minuto / por dia):** `gemini-3.6-flash` 5 / 20; `gemini-3.5-flash-lite` 15 / 500; `gemini-3.1-flash-lite` 15 / 500; `gemini-3.8-flash` 5 / 20; todos com 250 mil tokens por minuto. A quota é **por modelo**, então 429 no principal passa para o fallback. Em 19/11 o Google respondeu **503 "alta demanda"** com frequência (inclusive nos Flash Lite) e algumas chamadas ao principal ficaram sem resposta até o timeout de 25 s. A API trata assim: 429, 503, timeout e 404 passam logo para o próximo modelo, e o modelo que falhou fica 60 s de lado; só quando todos falham o usuário vê erro |
| **Entrada** | PDF/imagem enviados inline (base64) no `generateContent`. Leitura multimodal nativa, sem OCR próprio |
| **Saída** | `responseMimeType: application/json` + `responseSchema` |
| **Abstração** | Trait `BillExtractor` no crate `extract`. Trocar de provedor é implementar a trait, sem tocar no resto |
| **Atenção** | No free tier, o conteúdo **pode ser usado pelo Google** para melhorar produtos. Por isso há aviso, consentimento e fixtures como caminho padrão |

## Dados

| | |
|---|---|
| **Supabase** (Postgres, projeto `humanobill`, região **`ca-central-1`**, ~20 ms da API no Render Virginia) | Tabela `analyses`, `pg_cron` para expirar. Plano free (500 MB, que sobra) |
| Atenção | Projetos free do Supabase **pausam após ~1 semana sem atividade**. O keep-alive da API faz um `select 1` no `/health` para manter o banco ativo |

## Infra e deploy

| Peça | Onde | Custo | Observações |
|---|---|---|---|
| Front | **Vercel** (Hobby) | R$ 0 | Deploy automático a cada push na `main` |
| API | **Render** Web Service free (Docker, região `virginia`, a mais próxima do Brasil) | R$ 0 | 512 MB RAM. **Dorme após 15 min sem tráfego** e leva ~1 min para acordar. 750 h/mês **por workspace**: cobrem 1 serviço 24/7 (~744 h), não dois. Os docs não proíbem nem permitem ping de keep-alive; só suspendem serviço que *gera* tráfego anormal ([render.com/docs/free](https://render.com/docs/free)). Ping em `/robots.txt` não acorda o serviço. Use `/health` |
| Keep-alive | **UptimeRobot** (free, ping em `/health` a cada 5–10 min) + **GitHub Actions cron** como reserva | R$ 0 | O cron do Actions pode atrasar ou pular execuções, por isso fica só como reserva |
| Banco | Supabase free | R$ 0 | ver acima |
| IA | Gemini free tier | R$ 0 | quota limitada. Rate limit na API protege |
| CI | GitHub Actions | R$ 0 | `cargo fmt --check`, `clippy`, `test`, `audit`, build do web |
| E-mail | **Resend** (free tier) | R$ 0 | Lembretes (RF18). Precisa de domínio/remetente verificado (P11) |
| Cron dos lembretes | GitHub Actions (1×/dia) | R$ 0 | Chama `/internal/reminders/dispatch` com segredo |

**Build do Rust no Render:** compilar no free tier é lento. Usar um Dockerfile multi-stage com `cargo-chef` para cachear as dependências. Plano B: o GitHub Actions builda a imagem e publica no GHCR, e o Render só puxa a imagem pronta.

**Plano B de hospedagem:** se o Render free for instável ou se os termos impedirem keep-alive, migrar para o **Google Cloud Run** (free tier, cold start de ~1 s com Rust). O mesmo Dockerfile serve.

## Custo total previsto: **R$ 0/mês**
