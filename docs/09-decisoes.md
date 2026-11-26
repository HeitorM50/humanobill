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

## Decididas pelo Heitor (25/11/2026)

| # | Decisão | Por quê |
|---|---|---|
| D26 | **Fora da entrega de 30/11:** contas de água e gás (#56) e encaminhar fatura por e-mail (#57). **Open Finance (#59) fica só no pitch**, como visão de produto. As issues foram fechadas como "não planejado" | Faltam 5 dias e o risco agora é a entrega (vídeo, README, submissão). Água e gás pedem fixtures, regras e base legal novas; o e-mail inbound depende de domínio próprio; o Open Finance pede sandbox e credenciamento |
| D27 | **Sem domínio próprio.** A demo fica em `humanobill.vercel.app`, a API em `humanobill-api.onrender.com`, e o Resend fica em modo teste: o lembrete por e-mail (RF18) só chega no e-mail da conta, o que basta para mostrar o fluxo no vídeo. Fecha P01, P11 e #35 | Comprar e verificar um domínio (DNS, SPF, DKIM) a 4 dias da entrega é risco sem ganho na nota. *Revisável depois do evento* |

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
| C19 | **Marcação visual (#51):** cada item ganha `bbox` opcional (`page`, `x`, `y`, `w`, `h` de 0 a 1, origem no canto superior esquerdo). Para o Gemini, o schema e o prompt pedem a caixa no **formato nativo `box_2d`** (`[ymin, xmin, ymax, xmax]`, 0 a 1000), e o `crates/extract` converte. Caixa inválida é descartada sem falhar a extração. As regras não usam `bbox`. A imagem no front vem do PNG público do exemplo ou da foto enviada (só na memória da aba); PDF enviado fica sem marcação | Teste real em 20/11: com `x/y/w/h` de 0 a 1 o modelo omitiu a caixa nos 3 itens; com `box_2d` acertou as 3 linhas com erro de ~0,005. Desenhar PDF no navegador exigiria o pdf.js (dependência pesada) |
| C20 | **Painel coletivo (`GET /stats`, página `/painel`):** só análises `upload` não expiradas; **k-anonimato com k = 5** (grupo menor some, e o tipo de fatura com menos de 5 análises mostra "poucos casos ainda"); cada análise conta uma vez por regra e severidade; o percentual usa como base as análises do mesmo tipo de fatura; cache em memória de 5 min | Mostra valor do produto sem expor ninguém: com poucos casos, um número isolado poderia identificar quem enviou a fatura. A demo fica de fora porque são faturas fictícias e distorceriam os números |
| C21 | **"Minhas contas" sem login (#60, #52, #53):** histórico, protocolo e data da contestação ficam só no `localStorage` do navegador (até 50 contas), com "Remover" e "Apagar tudo". Prazo contado só com protocolo: telecom 30 dias corridos; energia 5 dias úteis (art. 408 da REN 1.000, pulando sábado e domingo, sem feriados). Vencido, mostra a escalada de `dispute.rs`. Alerta quando a conta da mesma empresa sobe 15% ou mais | Entrega histórico e acompanhamento sem login nem e-mail (o envio está em modo demo, P11). Nada sai do aparelho, então não há dado pessoal no servidor. Os prazos seguem o texto da contestação para a tela não contradizer a carta |
| C22 | **Extrato bancário (#54, 22/11/2026):** tipo `bank_statement` e categoria `tarifa` no core; migration `0003` troca o check de `analyses.kind` (uma migration para os tipos novos). Do extrato, a IA extrai **só o que o banco cobrou** (tarifas, pacote, seguros, juros e IOF do limite), com valor positivo; as movimentações da pessoa não entram. `due_date` = último dia do período; `total_cents` = soma das cobranças. Regras BAN-01 (pacote sem contrato), BAN-02 (tarifa de serviço essencial, com uma pergunta por serviço: "passou de 4 saques?") e BAN-03 (produto debitado sem contratação), validadas na Res. CMN 3.919/2010 e no CDC; saque em rede compartilhada fica no máximo `suspeito`. Escalada: SAC → ouvidoria → Banco Central → consumidor.gov.br; a contestação **pede** resposta em 7 dias, sem citar o decreto do SAC. `RULES_VERSION` passa a 0.6.0 | O extrato tem dado pessoal em quase toda linha (nomes no Pix, lojas, salário): extrair só as cobranças resolve a privacidade e basta para as regras. A cota grátis depende de quantas vezes a pessoa usou o serviço no mês, e contar saques pela IA erraria; a pergunta põe o fato na mão de quem sabe. O Decreto 13.135/2026 revogou o 11.034, e não está claro se vale para todo banco |
| C23 | **Fatura do cartão de crédito (#55, 22/11/2026):** tipo `card_bill`. A IA separa um a um só o que o emissor cobrou (anuidade e tarifas, seguros e serviços, multa, juros, IOF); compras, parcelas, pagamentos e saldo anterior viram **um único item** "Compras e lançamentos do período", com o valor que fecha o total. Regras CAR-01 (seguro ou serviço sem contratação, pode virar `indevido`), CAR-02 (tarifa fora da lista do cartão, sempre `suspeito`), CAR-03 (anuidade em cartão oferecido sem anuidade, `suspeito`), CAR-04 (multa acima de 2%, CDC art. 52, § 1º, `indevido`) e CAR-05 (soma). Sem regra de juros de mora (sem teto oficial) nem de rotativo e teto de 100% (precisam de mais de uma fatura) | Nome de loja e data de cada compra dizem muito sobre a pessoa e não servem para nenhuma regra. Juntar as compras num item mantém a soma igual ao total (validação do core) sem expor nada. A CAR-02 fica `suspeito` porque a Res. 3.919 lista o que pode cobrar, sem uma frase de vedação das demais |
| C24 | **Bot no WhatsApp com simulador (#58):** o bot existe no código no formato da **WhatsApp Cloud API** (`crates/api/src/routes/whatsapp.rs`), com o cliente da Graph API atrás de um trait (mock nos testes). **Desligado por padrão:** sem `WHATSAPP_VERIFY_TOKEN`, `WHATSAPP_APP_SECRET`, `WHATSAPP_TOKEN` e `WHATSAPP_PHONE_ID` as rotas do webhook respondem 404. O que a demo mostra é o **simulador** (`POST /whatsapp/simulador`, página `/simulador-whatsapp`), ligado em produção, que usa a mesma lógica de conversa mas só texto e exemplos (sem IA, sem arquivo) e o rate limit das rotas de escrita. A resposta do bot só repete o que o motor de regras decidiu (empresa e mês, total, quantos pontos, quanto "parece indevido", até 3 achados e o link `/r/<id>`); achado "suspeito" nunca vira "indevido". Telefone, texto e mídia nunca vão para log. O webhook responde 200 logo e processa em segundo plano, com limite por remetente (5 faturas e 20 exemplos por hora), limite global (40 faturas por hora) e descarte de entregas repetidas | Não há conta Meta, número aprovado nem token, mas o júri precisa ver o canal funcionando. O simulador prova a conversa sem gastar a quota do Gemini nem expor nada novo em produção. O webhook não usa rate limit por IP porque quem chama é a Meta (IPs compartilhados): a assinatura do corpo e os limites por remetente protegem a quota da IA. *Revisável: com o número aprovado, basta preencher as quatro variáveis no Render e cadastrar a URL do webhook no painel da Meta* |
