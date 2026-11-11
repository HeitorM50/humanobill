# 10 — Pontos em aberto

Os itens abaixo **não bloqueiam** o início. Cada um tem dono e prazo. Ao fechar um item, mova a decisão para o [09 — Decisões](./09-decisoes.md).

| # | Ponto | Dono | Prazo | Notas |
|---|---|---|---|---|
| P01 | ~~Nome do repositório~~ ✅ `HeitorM50/humanobill` (privado até 29/11). **Falta: domínio próprio?** | Heitor | 24/11 | O repo precisa virar público antes da submissão (item 6.1). Domínio ligado ao P11 |
| P02 | ~~Modelo Gemini~~ ✅ `gemini-3.6-flash`. **Falta: quotas do free tier**, que só aparecem no painel do AI Studio | Heitor | 13/11 | Define o rate limit da API (#40) |
| P03 | Validar as **regras de energia** no texto oficial da REN ANEEL 1.000/2021: acréscimos moratórios, faturamento por média, ciclo de leitura e devolução de cobrança indevida | Heitor (pesquisa) | 22/11 | Até lá, as regras ENE emitem só `suspeito` |
| P04 | Validar as **regras de telecom** no texto do RGC (Res. Anatel 765/2023): consentimento para SVA, contestação e cancelamento | Heitor (pesquisa) | 17/11 | Hoje a base é o Idec |
| P05 | ✅ Os termos do Render **não proíbem** ping de keep-alive. Só suspendem serviço que gera tráfego de saída anormal. Risco baixo, com plano B (Cloud Run) documentado | — | 11/11 | [render.com/docs/free](https://render.com/docs/free) |
| P06 | ✅ Supabase free: 2 projetos grátis por dono (pausados não contam), 500 MB por projeto. O `/health` faz `select 1` a cada ping, o que mantém o projeto ativo | — | 11/11 | [billing do Supabase](https://supabase.com/docs/guides/platform/billing-on-supabase) |
| P07 | Fazer o **rascunho do projeto no painel** do hackbrasil.com, que libera créditos Firecrawl/Devin/Nvoip | Heitor | 14/11 | Item 10.3. É opcional, mas grátis |
| P08 | Tabela de **bandeiras tarifárias** dos meses da demo (fixture) | Heitor | 25/11 | Usar os valores reais publicados pela ANEEL como referência |
| P09 | Cor de marca final e logo | Heitor | 27/11 | Proposta: verde-esmeralda |
| P10 | Formato e duração do **pitch na final** | Organização | 02/12 | Chega por e-mail com os finalistas |
| P11 | **Remetente do Resend:** sem domínio próprio, só dá para enviar ao e-mail da conta (modo teste). Para enviar a qualquer usuário, é preciso um domínio verificado | Heitor | 24/11 | Ligado ao P01 (domínio). Sem domínio, o RF18 funciona só na demo para o próprio e-mail |
| P12 | Gatilho do envio diário: **GitHub Actions cron** (escolhido) vs `pg_cron` + `pg_net` no Supabase | Heitor | 26/11 | Actions é mais simples. O pg_cron não depende do repo |
