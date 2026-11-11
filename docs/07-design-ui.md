# 07 — Design e UI

## Direção visual: "fintech clean e confiante"

Referências de *sensação*, não para copiar: Nubank, Linear, Mercury. Bastante espaço em branco, tipografia forte, uma cor de destaque e números grandes.

**Princípios:**
1. **O número é o herói.** "R$ 47,80 parece indevido" é o primeiro elemento que o olho encontra.
2. **Clareza antes de estética.** Cada tela responde a uma pergunta só.
3. **Confiança por transparência.** Toda acusação mostra o porquê e a lei, nunca uma "caixa-preta".
4. **Mobile-first.** Grande parte das faturas é vista e fotografada pelo celular.
5. **Humano.** O tom é de conversa: "Isso é uma taxa de...". Nada de juridiquês.

## Tokens (proposta inicial, ajustar no polish)

| Token | Uso |
|---|---|
| `--bg` / `--surface` | Fundo neutro claro e cartões brancos. Dark mode via `prefers-color-scheme` |
| `--accent` | **Uma** cor de marca (proposta: verde-esmeralda, "dinheiro de volta") |
| `--danger` | Achado `indevido` (vermelho sóbrio, não alarmista) |
| `--warning` | Achado `suspeito` (âmbar) |
| `--ok` | Item conferido (verde suave) |
| Tipografia | Sans geométrica (ex.: Inter ou Geist) com números tabulares para valores |
| Raio / sombra | Raio médio (12–16 px), sombras sutis |

Severidade **nunca** depende só da cor: tem ícone + rótulo de texto (acessibilidade).

## Telas

### 1. Landing `/`
- Headline: **"Sua conta, em português humano."**
- Subtítulo: "Descubra em segundos se você está pagando o que não deve, e conteste com o texto pronto."
- CTA principal: **Testar com fatura de exemplo** (escolher telecom ou luz). CTA secundário: **Enviar minha fatura**.
- "Como funciona" em 3 passos (enviar → entender → contestar) + selo "seus dados pessoais não são guardados".
- Dispara o warm-up (`GET /health`) ao carregar.

### 2. Enviar `/analisar`
- Dropzone com "tirar foto" no celular.
- Aviso do free tier + checkbox de consentimento (obrigatório).
- Progresso em etapas: *Lendo sua fatura…* → *Conferindo item por item…* → *Pronto*.

### 3. Resultado `/r/:id`
- **Cabeçalho:** operadora, mês e total. Cartão destacado: **"R$ X parecem indevidos · até R$ Y recuperáveis"**, com o contador animado.
- **Perguntas rápidas** (se houver): "Você contratou *Clube Vivaz Música*?" Sim/Não. "Já pagou esta fatura?"
- **Achados** em cartões: severidade, título, explicação, valor, base legal (link) e checkbox "incluir na contestação".
- **Sua conta explicada:** lista de itens com a frase "o que é isso" e status (ok/suspeito/indevido).
- **Energia:** mini-gráfico do histórico de 12 meses com o mês atual destacado.
- Link "Os dados estão errados? Corrigir" abre a tela de conferência.
- Rodapé: "Este resultado expira em 30 dias" + copiar link.

### 4. Conferir dados (modal ou drawer)
- Campos extraídos editáveis. Salvar reavalia na hora (sem IA).

### 5. Contestação (seção ou passo do resultado)
- Texto gerado num bloco com botão **Copiar** (com feedback de "copiado!").
- Passo a passo numerado com links (SAC → Anatel/ANEEL → consumidor.gov.br) e os prazos.
- Aviso: "conteúdo informativo, não é orientação jurídica".

### 5b. Roteiro de ligação (RF17)
- Toggle "Prefiro ligar": um roteiro numerado em letras grandes + um campo "anote o protocolo aqui", que depois aparece no texto da contestação.

### 5c. Lembrete (RF18/RF19)
- Cartão "Quer que a gente te lembre do prazo?" com campo de e-mail, checkbox de consentimento separado e a frase "seu e-mail é apagado depois do aviso".

### 5d. Comparar faturas (RF16)
- Na tela de resultado de telecom: "Comparar com o mês anterior" → envio da 2ª fatura → lista de itens **novos**, **removidos** e **alterados** + variação do total.

### 5e. Composição da conta (RF23)
- Barra empilhada ou donut: energia/plano · impostos · bandeira · iluminação pública · **extras** (cor de alerta).

### 6. Privacidade `/privacidade`
- Uma página curta e honesta: o que é processado, por quem, por quanto tempo e o que nunca guardamos.

## Acessibilidade para idosos (D12)

| Recurso | Comportamento |
|---|---|
| **A+** (RF20) | Botão fixo no header. Texto base ≥ 20 px, botões ≥ 56 px. Preferência salva no `localStorage` |
| **Ouvir** (RF21) | Botão 🔊 no resumo e em cada achado. `speechSynthesis` em pt-BR. Escondido se o navegador não suportar |
| **Modo passo a passo** (RF22) | Opção na landing. Uma etapa por tela, botões "Próximo"/"Voltar" grandes, sem termos técnicos |

## Estados obrigatórios

| Estado | Comportamento |
|---|---|
| Carregando a API (cold start) | Skeleton + "acordando o servidor…" depois de 2 s |
| Arquivo inválido / grande demais | Mensagem inline no dropzone |
| Extração falhou | "Não consegui ler bem essa fatura. Tente um PDF ou uma foto mais nítida", com um botão para usar o exemplo |
| Nenhum problema encontrado | Celebração discreta: "Tudo certo com esta conta 🎉" + itens explicados |
| Resultado expirado | 410 → "Este resultado expirou. Analise de novo." |
| Rate limit | "Muitas análises seguidas, tente em alguns minutos" |

## Copy: tom de voz

- **Sim:** "Isso parece um serviço extra que você pode não ter contratado."
- **Não:** "Foi identificada irregularidade na cobrança do item supramencionado."
- **Nunca:** "Você vai ganhar R$ X", "garantido", "advogado".
