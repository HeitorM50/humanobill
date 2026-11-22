# 09 — Log de decisões

Formato: **decisão**, quem decidiu, data e por quê. Quando uma decisão mudar, não apague a linha: marque como *substituída* e adicione a nova.

## Decididas pelo Heitor (10/11/2026)

| # | Decisão | Por quê |
|---|---|---|
| D01 | Ideia: **HumanoBill**, analisador e contestador de faturas | A dor é universal e tem dados fortes. A demo é visual. O nome já existia na pasta |
| D02 | Trilha: **Pagamentos & finanças** | Gestão financeira pessoal. O resultado central é dinheiro |
| D03 | MVP cobre **Telecom + Energia** | Mais impacto. Mostra que a arquitetura é extensível |
| D04 | Front em **Vite + React + TS**, visual **fintech clean e confiante** *(a parte visual foi substituída pela D23)* | Onde já é produtivo, e tem visual moderno |
| D05 | Back em **Rust (Axum)** | Quer explorar Rust. Já fez projetos pequenos |
| D06 | Banco: **Supabase** (Postgres) | Postgres gerenciado e free. MCP já conectado |
| D07 | IA: **Gemini, só free tier**, com aviso + consentimento | Custo zero |
| D08 | **Sem login** + modo demo | Os jurados não precisam criar conta (item 7.4) |
| D09 | Canal: **web responsiva** | O menor custo para o MVP. WhatsApp e e-mail ficam no roadmap |
| D10 | API no **Render free + keep-alive** | Custo zero. O Fly.io não tem mais free tier |
| D11 | Faturas de demo **sintéticas com marcas fictícias** | Item 6.9 + zero risco de imitar documento de empresa real |
| D12 | **Arquivo nunca persistido.** Resultado anonimizado por **30 dias** | LGPD. 30 dias = prazo de resposta da operadora |
| D13 | Contestação: **texto copiável + passo a passo** (PDF na expansão) | Escopo |
| D14 | Nome do produto: **HumanoBill** | — |
| D15 | Lembrete por **e-mail opcional** (Resend), com consentimento separado. **E-mail apagado após o envio** | Dor D10 ("alguém me lembrar"). Minimização LGPD |
| D16 | **Comparar até 2 faturas** de telecom no MVP (Should) | Dor D02. O telecom não traz histórico impresso |
| D17 | Destaque do item = **trecho original transcrito** + página (marcação visual fica para a final) | Barato e confiável. Atende a D11 |
| D18 | Acessibilidade: **A+, ouvir e modo guiado** no MVP (Should) | Dor D12: "eles mesmos usariam" |
| D19 | Priorização **MoSCoW** | O escopo cresceu ~5 dias. Os cortes ficam definidos de antemão |
| D20 | Critérios BDD em **Gherkin PT** | Legível para o Heitor e para os jurados |
| D21 | Contestação cobre **todos os canais**: texto (chat/WhatsApp/e-mail/formulário/órgãos) + **roteiro de ligação** | Entrevista: todos os canais marcados |
| D22 | Regras **TEL-01, 03, 04, 06 e 08 validadas** em fonte oficial (CDC no Planalto; RGC pelas páginas e pelo Manual Operacional da Anatel). TEL-03/04 citam o **RGC art. 78, § 1º**, e não o CDC art. 52 ou o CC art. 406, que não se aplicam diretamente | Pesquisa de 11/11/2026 (P04). O texto integral do RGC estava fora do ar: reconferir os arts. 48, 78 § 1º e 83 quando ele voltar |

## Decididas pelo Heitor (16/11/2026)

| # | Decisão | Por quê |
|---|---|---|
| D23 | Direção visual **"Marca-texto em blocos"**: estilo de papel, caneta azul e marca-texto amarelo, com as informações em blocos de cor de ponta a ponta. Fontes Bricolage Grotesque + Atkinson Hyperlegible Next. Logo **"recibo grifado"** e ilustrações de linha. Guia completo no [doc 07](./07-design-ui.md) | Comparadas quatro direções em pranchetas. Esta junta uma identidade própria do produto (a conta anotada) com a leitura mais direta. Substitui a parte visual da D04 |
| D24 | Frase de apresentação: **"Sua conta, em português claro."** *(substitui "em português humano")* | "Português humano" soava estranho. O nome HumanoBill já carrega a ideia |

## Decididas pelo Heitor (17/11/2026)

| # | Decisão | Por quê |
|---|---|---|
| D25 | Na composição da conta (RF23), multa e juros por atraso (`encargo_atraso`) ficam em **"Outros"**, sem fatia própria. Partes zeradas não aparecem | O RF23 não lista "Encargos" entre as partes, e uma fatia a mais deixaria o gráfico mais difícil de ler |

## Decisões técnicas complementares (revisáveis)

| # | Decisão | Por quê |
|---|---|---|
| C01 | Monorepo público único: `apps/web` + `crates/{core,rules,extract,api}` + `fixtures/` | Um link só para o júri. Histórico unificado |
| C02 | Licença **MIT** | Recomendada pelo item 6.6. Permite aos jurados ler e rodar |
| C03 | **A IA extrai e explica. O motor de regras decide** | Evita o "wrapper de LLM". É determinístico e testável |
| C04 | Schema de extração **sem** nome/CPF/endereço | Privacidade by design |
| C05 | Valores em **centavos `i64`** | Nunca usar float com dinheiro |
| C06 | 3 severidades. Regra "a validar" nunca emite `indevido` | Evita falso positivo grave |
| C07 | **1 chamada de IA por análise.** Correções e respostas reavaliam só o motor | Economiza o free tier e deixa a resposta instantânea |
| C08 | Modo demo usa **JSON pré-extraído** (sem IA) + botão "com IA ao vivo" | A demo nunca cai por quota ou por falha do Gemini |
| C09 | Tipos TS gerados com **ts-rs** | Front e back sempre sincronizados |
| C10 | Gemini via **REST (reqwest)** atrás da trait `BillExtractor` | Não há SDK oficial em Rust. Fica fácil trocar de provedor |
| C11 | Front **nunca** acessa o Supabase direto | Superfície de ataque menor |
| C12 | UptimeRobot como keep-alive principal, Actions cron como reserva | O cron do Actions pode atrasar ou ser pulado |
| C13 | `/health` faz `select 1` no banco | Evita a pausa por inatividade do Supabase free |
| C14 | Plano B de hospedagem: **Google Cloud Run** | O mesmo Dockerfile serve. Free tier com cold start curto |
| C15 | Submeter até as **12h de 30/11** | Buffer contra imprevistos |
| C16 | O roteiro de ligação (RF17) sai **no mesmo `POST /analyses/:id/dispute`**, no campo `call_script`, e não numa rota `/call-script` | Contestação e roteiro usam os mesmos achados selecionados. Uma chamada só, e o protocolo anotado já entra nos dois |
| C17 | O **protocolo** da contestação aceita só **dígitos e hífen, até 30 caracteres** *(antes: letras, `. / espaço`, até 40)*. Nunca vai para log nem para mensagem de erro | Protocolos de SAC são numéricos. A regra estrita barra injeção e vazamento. *Revisável: se alguma operadora usar letras, abrir a regra de novo* |
| C18 | **Tabelas da ANEEL geradas do dado aberto** (`scripts/aneel`) e embutidas no motor com `include_str!`. **ENE-06 conservadora** (20/11/2026): só para as distribuidoras da tabela (nome inteiro normalizado), com impostos destacados, tarifa convencional e todo o ciclo coberto por vigências; folga de 2%; sempre `suspeito`, status "a validar". `RULES_VERSION` passa a 0.5.0 | Substitui a nota do doc 03 que deixava a ingestão do dado aberto para a expansão. Sem I/O em execução (C03). A demo (Luzia Energia, fictícia) não muda de resultado. Falso positivo custa caro: a fatura não diz a subclasse (baixa renda, rural) nem se o preço da linha já inclui PIS/COFINS |
| C20 | **Painel coletivo (`GET /stats`, página `/painel`):** só análises `upload` não expiradas; **k-anonimato com k = 5** (grupo menor some, e o tipo de fatura com menos de 5 análises mostra "poucos casos ainda"); cada análise conta uma vez por regra e severidade; o percentual usa como base as análises do mesmo tipo de fatura; cache em memória de 5 min | Mostra valor do produto sem expor ninguém: com poucos casos, um número isolado poderia identificar quem enviou a fatura. A demo fica de fora porque são faturas fictícias e distorceriam os números |
