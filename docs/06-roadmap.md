# 06 — Roadmap

> Regras do regulamento que moldam este cronograma: repo novo depois de 04/11 às 21h (6.1) · o júri avalia o último commit até **30/11 às 23h59** (7.1) · **zero commits de 01/12 a 11/12** (8.4) · demo e repo no ar até pelo menos 15/12 (6.7, 7.3).

## Princípio de priorização

Na ordem de prioridade:
1. **O fluxo demo funciona de ponta a ponta no ar** (execução técnica, o critério de desempate).
2. As regras de telecom estão sólidas e testadas.
3. As regras de energia vêm em seguida.
4. Polish visual e vídeo.

**Prioridade dos requisitos (MoSCoW):** ver [02 — Requisitos](./02-requisitos.md). Os Must vêm primeiro, os Should entram nas semanas 2 e 3, e os Could (RF24, RF25, RF26/ENE-06) só entram se sobrar tempo.

**Se atrasar, cortar nesta ordem:** RF22 (modo guiado) → RF17 (roteiro de ligação) → RF16 (comparar) → RF18/RF19 (lembrete) → RF23 (composição) → ENE-05 (bandeira) → RF21 (ouvir) → RF20 (A+) → animações.

> ⚠️ A semana 3 ficou apertada com os Should. Se o marco da S2 atrasar mais de 1 dia, aplicar os cortes acima **antes** de começar a semana 3.

## Acompanhamento no GitHub

O progresso é controlado pelas **issues** do repositório:
- [📊 Painel do MVP](https://github.com/HeitorM50/humanobill/issues/65): issue fixada com o checklist de todas as issues por prioridade e semana.
- [Milestones](https://github.com/HeitorM50/humanobill/milestones): S1, S2, S3, Final e Expansão, com a % concluída de cada uma.
- Labels: `MVP` + `prioridade: must`, `prioridade: should`, `prioridade: could`, `expansão` e `ideia`.
- Ideias novas entram pelo template **💡 Ideia** e ficam como expansão até serem priorizadas.

Cada commit que conclui uma issue usa `Closes #n`. A coluna Status abaixo espelha o estado das issues.

## Ordem de implementação dos requisitos

A ordem segue as **dependências técnicas** e a prioridade **MoSCoW**. Um RF só começa quando os RFs de que ele depende estão prontos. Status: ⬜ pendente · 🟨 em andamento/parcial · ✅ pronto (todos os cenários BDD cobertos).

| # | Requisito | MoSCoW | Depende de | Semana | Status |
|---|---|---|---|---|---|
| 1 | Infra base (monorepo, CI, deploy, banco) + RNF06/RNF13/RNF15 desde o início | — | — | S1 | 🟨 |
| 2 | **RF04** Extrair dados da fatura (telecom) | Must | 1 | S1 | ⬜ |
| 3 | **RF01** Analisar fatura de exemplo (backend + fixtures) | Must | 2 | S1 | ⬜ |
| 4 | **RF02** Enviar fatura (API) + **RF03** Registrar consentimento (API) | Must | 2 | S1 | ⬜ |
| 5 | **RF07** Detectar cobranças indevidas (regras TEL) | Must | 2 | S2 | ⬜ |
| 6 | **RF08** Justificar achado + **RF09** Responder perguntas de confirmação | Must | 5 | S2 | ⬜ |
| 7 | **RF06** Explicar itens | Must | 2 | S2 | ⬜ |
| 8 | **RF01/RF02/RF03** telas (landing, demo, upload, consentimento) | Must | 3, 4 | S2 | ⬜ |
| 9 | Tela de resultado (RF06–RF09 no front) | Must | 5–8 | S2 | ⬜ |
| 10 | **RF10** Calcular valor recuperável | Must | 6 | S2 | ⬜ |
| 11 | **RF11** Gerar contestação + **RF12** Orientar contestação | Must | 10 | S2 | ⬜ |
| 12 | **RF17** Gerar roteiro de ligação | Should | 11 | S2 | ⬜ |
| 13 | **RF04 + RF07** para energia (extração + regras ENE) | Must | 5 | S2/S3 | ⬜ |
| 14 | **RF23** Exibir composição da conta | Should | 13 | S3 | ⬜ |
| 15 | **RF05** Corrigir dados extraídos | Must | 9 | S3 | ⬜ |
| 16 | **RF13** Reabrir resultado + **RF14** Expirar resultados | Must | 9 | S3 | ⬜ |
| 17 | **RF15** Exibir política de privacidade | Must | — | S3 | ⬜ |
| 18 | **RF20** Ampliar interface + **RF21** Ouvir explicação | Should | 9 | S3 | ⬜ |
| 19 | **RF16** Comparar faturas | Should | 9 | S3 | ⬜ |
| 20 | **RF18** Agendar lembrete + **RF19** Cancelar lembrete | Should | 16 | S3 | ⬜ |
| 21 | **RF22** Guiar análise | Should | 9, 11 | S3 | ⬜ |
| 22 | RNFs de fechamento: RNF07 (injection), RNF08 (a11y), RNF01/02 (desempenho) | — | tudo | S3 | ⬜ |
| 23 | **RF24**, **RF25**, **RF26** (Could) | Could | 13 | só se sobrar tempo | ⬜ |

## Fase online (10/11 → 30/11)

### Semana 1 — Fundação (10/11 ter → 16/11 seg)

| Dia | Entregas | Requisitos |
|---|---|---|
| 10/11 ter | ✅ Docs. Criar o repo novo no GitHub (**privado durante o desenvolvimento, tornar público antes de 30/11**), com LICENSE MIT, README inicial e `.gitignore`. Esqueleto do monorepo | — |
| 11/11 qua | CI básico (fmt, clippy, test, build web). Tipos de domínio no `core` (`ExtractedBill`, `BillItem`, `Finding`…) + export ts-rs | RNF14 |
| 12/11 qui | **Deploy do hello world:** API `/health` no Render (Docker + cargo-chef), web na Vercel chamando `/health`, projeto Supabase + migration `analyses`. UptimeRobot configurado | RNF03, RNF15 |
| 13/11 sex | Schema de extração de telecom (sem campos pessoais) + validação (soma, datas) | RF04, RNF04 |
| 14/11 sáb | **Faturas sintéticas de telecom** (3 cenários, ver doc 08) + o `extracted/*.json` esperado de cada uma | RF01 |
| 15/11 dom | Crate `extract`: cliente Gemini (PDF inline + `responseSchema`), retry com prompt de correção. Teste live com as 3 faturas | RF04 |
| 16/11 seg | `POST /analyses` (MIME, tamanho, `consent`) e `POST /analyses/demo/:sample` gravando no banco. Logs sem conteúdo | RF01, RF02, RF03, RNF06, RNF13 |

**Marco S1:** uma fatura de telecom enviada pela API, extraída pelo Gemini e salva no banco, tudo no ar.

### Semana 2 — Núcleo (17/11 ter → 23/11 seg)

| Dia | Entregas | Requisitos |
|---|---|---|
| 17/11 ter | Crate `rules`: estrutura `Rule`/`Finding` + TEL-02, TEL-07 + testes. Explicações por item vindas da extração | RF07, RF06, RNF11 |
| 18/11 qua | TEL-01 (SVA + pergunta), TEL-03/04, TEL-05, TEL-06, TEL-08 + justificativa (trecho + base legal) + `PATCH` com respostas + golden das 3 faturas | RF07, RF08, RF09, RNF10 |
| 19/11 qui | Front: landing + botão demo + upload com consentimento + progresso + warm-up | RF01, RF02, RF03, RNF02 |
| 20/11 sex | Front: tela de resultado (resumo, itens explicados, achados com justificativa, perguntas) | RF06, RF08, RF09 |
| 21/11 sáb | Recuperável (pago/não pago) + `POST /dispute` + tela de contestação (copiar + passo a passo) + roteiro de ligação | RF10, RF11, RF12, RF17, RNF12 |
| 22/11 dom | **Energia:** schema estendido + 3 faturas sintéticas de energia + JSONs esperados + extração testada | RF04, RF01 |
| 23/11 seg | Buffer / correções. **Telecom 100% de ponta a ponta no ar** | — |

**Marco S2:** um jurado consegue clicar em "exemplo de telecom" e chegar até a contestação copiada (todos os Must de telecom prontos).

### Semana 3 — Completar e polir (24/11 ter → 30/11 seg)

| Dia | Entregas | Requisitos |
|---|---|---|
| 24/11 ter | Regras ENE-01 a 05, 07 e 08 + testes + golden + passo a passo de energia + composição da conta | RF07, RF12, RF23 |
| 25/11 qua | Tela "confira os dados", `/r/:id` + expiração (410 + limpeza), `/privacidade`, A+ e ouvir | RF05, RF13, RF14, RF15, RF20, RF21, RNF05 |
| 26/11 qui | Comparar 2 faturas + lembrete por e-mail (Resend + cron) com cancelamento | RF16, RF18, RF19 |
| 27/11 sex | Modo guiado + **polish de UI** (responsivo, a11y, animações, estados de erro) + rate limit + teste de prompt injection | RF22, RNF06, RNF07, RNF08, RNF09 |
| 28/11 sáb | **README do repo** (problema, solução, arquitetura, como rodar, screenshots, GIF). Gravar o vídeo (tomada 1). Medir desempenho | RNF01, RNF02 |
| 29/11 dom | Editar o vídeo final (≤ 3 min). Revisão do checklist de submissão. **Tornar o repo público.** Teste em outro celular/navegador | RNF16 |
| **30/11 seg** | **Submeter até as 12h** (11h59 de buffer). Commit final antes das 23h59. Conferir se o vídeo abre sem login | — |

## Congelamento (01/12 → 11/12): nenhum commit

- Não alterar o código. Só manter a demo no ar (o keep-alive roda sem commit).
- 02/12: sai a lista de finalistas. **04/12 às 23h59: confirmar presença.**
- Preparar slides, roteiro do pitch e ensaios. Ler as regras da final que chegam por e-mail.
- Planejar o que construir **no dia** da final (veja abaixo).

## Final (12/12, São Paulo)

O desenvolvimento recomeça no local, nos mesmos repositórios. Escolher 1 feature de impacto visual que dê para fazer em poucas horas, por exemplo:
- **Histórico de N faturas** com gráfico de "quanto você pagou a mais no ano", ou
- **PDF formal da contestação**, ou
- **Encaminhar fatura por e-mail** (Resend inbound).

## Expansão pós-evento (visão de produto, também usada no pitch)

| Fase | O que entra |
|---|---|
| **v1.1 — Hábito** | Conta de usuário, histórico entre faturas (comparação com N meses), alerta de "sua conta subiu X%", lembretes recorrentes |
| **v1.2 — Mais contas** | Extrato bancário (tarifas vs pacote de serviços essenciais gratuitos), cartão de crédito (seguros e anuidade), água e gás |
| **v1.3 — Zero atrito** | Encaminhar o e-mail da fatura, bot no WhatsApp, Open Finance para extratos |
| **v1.4 — Até o fim** | Acompanhar protocolo e prazos, escalada guiada para Anatel/ANEEL/consumidor.gov, PDF formal |
| **v2 — Impacto coletivo** | Painel anonimizado do tipo "operadora X cobrou SVA em Y% das faturas", relatórios para Procon/Idec, dados abertos ANEEL integrados (tarifas e bandeiras reais) |
| **Negócio** | Freemium (análise grátis e acompanhamento pago) ou success fee sobre o valor recuperado. B2B2C com Procons e associações de consumidores |
