# 07 — Design e UI

Este é o guia de estilo e o design system do HumanoBill. Ele vale para todas as telas de `apps/web`.

| Arquivo | Para que serve |
|---|---|
| [`design/tokens.css`](./design/tokens.css) | Tokens prontos para o Tailwind v4 (`@theme`). Substitui o bloco de tokens de `apps/web/src/index.css` |
| [`design/referencia.html`](./design/referencia.html) | Pranchetas da direção aprovada. Abra no navegador para ver a aparência esperada em celular e desktop |
| [`design/logo-simbolo.svg`](./design/logo-simbolo.svg) · [`logo-simbolo-negativo.svg`](./design/logo-simbolo-negativo.svg) | Símbolo do logo, para fundo claro e para fundo azul |
| [`design/ilustracoes/`](./design/ilustracoes) | Ilustrações dos três passos, do estado "tudo certo" e a caneta marca-texto |

A referência HTML mostra a **aparência**. Se ela e este documento divergirem, vale este documento.

## 1. Direção visual: "Marca-texto em blocos"

Decisão **D23** (16/11/2026). Substitui a direção "fintech clean" inicial.

**Conceito.** Alguém de confiança pega a sua fatura, passa marca-texto nas linhas erradas e escreve ao lado o que cada uma significa. O visual vem desse gesto: papel, caneta azul e marca-texto amarelo. As informações são dispostas em blocos de cor chapada de ponta a ponta, com o valor em destaque no topo.

**Personalidade.** Direto, claro e do lado do consumidor. Firme sem ser agressivo. Nada de enfeite.

**Frase de apresentação:** **"Sua conta, em português claro."** (decisão D24; substitui "em português humano").

**Princípios:**
1. **O número é o herói.** "R$ 22,80 parece indevido" é o primeiro elemento que o olho encontra.
2. **O amarelo é raro.** O marca-texto só aparece em dois lugares: no que é indevido ou importante, e na ação principal sobre um bloco azul. Se tudo é grifado, nada é.
3. **A conta parece uma conta.** Os itens aparecem como uma fatura em papel, e não como uma pilha de cartões.
4. **Confiança por transparência.** Toda acusação mostra o porquê e a lei.
5. **Mobile-first.** Grande parte das faturas é vista e fotografada pelo celular.
6. **Legível para todos.** Texto em fonte feita para baixa visão, alvos de toque grandes, contraste AA.

**Referências de sensação** (não copiar): gov.uk pela clareza, Monzo e Wise pelos blocos de cor, anotações de leitura (Readwise) pelo marca-texto.

## 2. Cores

Uma cor de marca (azul de caneta), uma cor de destaque (marca-texto) e as semânticas. Os valores abaixo estão em [`design/tokens.css`](./design/tokens.css).

### Base

| Token | HEX | RGB | Uso |
|---|---|---|---|
| `bg` | `#FBFAF7` | 251, 250, 247 | Fundo da página (papel) |
| `surface` | `#FFFFFF` | 255, 255, 255 | Cartões, fatura, campos |
| `ink` | `#161B33` | 22, 27, 51 | Texto principal |
| `muted` | `#545A75` | 84, 90, 117 | Texto secundário |
| `line` | `#DEDCD3` | 222, 220, 211 | Divisores e bordas decorativas |
| `field-line` | `#8A8FA6` | 138, 143, 166 | Borda de campo de formulário |

### Marca e destaque

| Token | HEX | RGB | Uso |
|---|---|---|---|
| `accent` (primária) | `#1B2A6B` | 27, 42, 107 | Botão principal, links, blocos de cor, cabeçalho |
| `accent-strong` | `#121D4D` | 18, 29, 77 | Hover e pressionado da primária |
| `accent-soft` | `#E8EBF7` | 232, 235, 247 | Fundo suave, hover do botão secundário |
| `on-accent` | `#FFFFFF` | 255, 255, 255 | Texto sobre a primária |
| `highlight` (destaque) | `#FFE45C` | 255, 228, 92 | Marca-texto e botão principal sobre azul |
| `highlight-strong` | `#F5D63A` | 245, 214, 58 | Hover do botão de destaque |
| `on-highlight` | `#161B33` | 22, 27, 51 | Texto sobre o amarelo |

Não há cor secundária de marca. O papel de "secundária" é feito pelo par `accent-soft` + `highlight`.

### Semânticas

| Papel | Texto | Fundo suave | Borda | Quando |
|---|---|---|---|---|
| Erro / `indevido` (`danger`) | `#C0271D` (192, 39, 29) | `#FBE9E7` | `#EFB3AD` | Achado indevido, erro de formulário |
| Atenção / `suspeito` (`warning`) | `#8A5A00` (138, 90, 0) | `#FFF3C9` | `#E6CD7A` | Achado suspeito, prazos, aviso do free tier |
| Sucesso / `ok` (`ok`) | `#1F6B45` (31, 107, 69) | `#E3F3EA` | `#A9D6BC` | Item conferido, "tudo certo", "copiado" |
| Informação (`info`) | `#1B2A6B` (27, 42, 107) | `#E8EBF7` | `#B9C1E4` | Explicações neutras ("por que isso foi marcado?") |

### Contraste (medido)

| Par | Razão |
|---|---|
| `ink` sobre `bg` | 16,2:1 |
| `muted` sobre `bg` / `surface` | 6,5:1 / 6,8:1 |
| branco sobre `accent` | 13,2:1 |
| `ink` sobre `highlight` | 13,3:1 |
| `highlight` sobre `accent` | 10,4:1 |
| `danger` sobre `danger-soft` | 5,1:1 |
| `warning` sobre `warning-soft` | 5,3:1 |
| `ok` sobre `ok-soft` | 5,6:1 |
| `field-line` sobre `surface` (borda) | 3,2:1 |

### Regras de cor

- **Nunca** usar a paleta crua do Tailwind (`slate-*`, `emerald-*`, `amber-*`, `red-*`). Só tokens.
- Amarelo nunca leva texto branco, e nunca é usado como cor de texto sobre fundo claro.
- Severidade **nunca** depende só da cor: sempre ícone + rótulo de texto.
- **Tema único (claro).** Não há modo escuro nesta versão.

## 3. Tipografia

| Papel | Família | Motivo |
|---|---|---|
| Títulos e valores | **Bricolage Grotesque** (700) | Tem personalidade sem perder seriedade. Os números ficam fortes |
| Texto e interface | **Atkinson Hyperlegible Next** (400, 600, 700) | Desenhada para baixa visão: letras difíceis de confundir. Coerente com o modo idoso |
| Trecho original da fatura | Monoespaçada do sistema | Mostra que o texto foi transcrito da fatura |

As duas famílias são livres (SIL Open Font License). **Auto-hospedar** os arquivos (pacotes `@fontsource-variable/*` ou `.woff2` em `public/fonts`), com `font-display: swap`. Não chamar o Google Fonts em produção: a página de privacidade promete o mínimo de terceiros.

### Escala

| Nível | Celular → desktop | Altura de linha | Peso | Família | Uso |
|---|---|---|---|---|---|
| Valor herói (`text-hero`) | 44 → 68 px | 1,0 | 700 | Bricolage | O valor indevido no bloco do resultado |
| Display (`text-display`) | 36 → 64 px | 1,05 | 700 | Bricolage | Título da tela principal |
| H1 (`text-h1`) | 32 → 40 px | 1,1 | 700 | Bricolage | Título de página |
| H2 (`text-h2`) | 26 → 30 px | 1,15 | 700 | Bricolage | Título de seção |
| H3 (`text-h3`) | 22 px | 1,25 | 700 | Bricolage | Título de cartão ou de grupo |
| H4 (`text-h4`) | 18 px | 1,3 | 700 | Atkinson | Título de achado, rótulo forte |
| Body (`text-body`) | 17 px | 1,5 | 400 | Atkinson | Texto corrido |
| Caption (`text-caption`) | 14 px | 1,4 | 400 | Atkinson | Notas e legendas. É o menor tamanho permitido |

### Regras de tipografia

- Só três pesos: **400** (texto), **600** (botões, rótulos de campo, selos) e **700** (títulos, valores, totais).
- Espaçamento entre letras de `-0.02em` nos títulos em Bricolage. Texto normal sem ajuste.
- Números sempre tabulares (`tabular-nums`).
- Sem rótulos em caixa alta. Sem texto abaixo de 14 px.
- Texto corrido com no máximo 65 caracteres por linha.
- Todos os tamanhos em `rem`, para o modo A+ escalar tudo.

## 4. Espaçamento e layout

**Escala de espaçamento** (base 4 px): 4 · 8 · 12 · 16 · 20 · 24 · 32 · 40 · 56 · 72. No Tailwind: `1, 2, 3, 4, 5, 6, 8, 10, 14, 18`. Não usar valores fora da escala.

| Item | Valor |
|---|---|
| Largura máxima do conteúdo | 1120 px (`max-w-page`), centralizado |
| Largura máxima de texto corrido | 65ch (`max-w-text`) |
| Margem lateral | 20 px no celular, 40 px a partir de 768 px |
| Espaço vertical dentro de um bloco de cor | 28–32 px no celular, 44–56 px no desktop |
| Espaço entre seções | 32 px no celular, 56 px no desktop |
| Espaço dentro de cartão | 16 px (20 px em cartões largos) |
| Espaço entre elementos irmãos | 12–16 px |
| Raio de cartão, fatura, dropzone, alerta | 8 px (`rounded-card`) |
| Raio de botão, campo, selo | 6 px (`rounded-control`) |
| Altura mínima de alvo de toque | 48 px (44 px para links em linha). No modo A+, 56 px |

**Grid.** Uma coluna no celular. A partir de 768 px, duas colunas de mesma largura com 24 px de espaço entre elas. A tela principal usa a proporção 1,1 : 0,9 (texto : ilustração).

**Blocos de ponta a ponta.** O cabeçalho, o bloco de abertura da tela principal e o bloco do valor no resultado têm fundo `accent` de borda a borda da janela. O conteúdo dentro deles respeita a largura máxima de 1120 px. Para isso, o `<main>` deixa de ter `max-w-3xl`: cada seção controla a própria largura.

**Alinhamento.** Tudo alinhado à esquerda. Centralizado só no dropzone e em estados vazios. Valores em dinheiro alinhados à direita nas linhas da fatura.

## 5. Componentes

### 5.1 Botões

Base comum: altura mínima 48 px, espaço interno 10 × 20 px, raio 6 px, peso 600, texto de 17 px, ícone opcional com 8 px de distância.

| Variante | Aparência | Uso |
|---|---|---|
| **Primary** | Fundo `accent`, texto branco | A ação principal de uma área clara ("Analisar", "Escolher arquivo") |
| **Secondary** | Fundo `surface`, borda 1,5 px `accent`, texto `accent` | Ação alternativa ("Tirar foto", "Enviar minha fatura" em área clara) |
| **Tertiary (destaque)** | Fundo `highlight`, texto `ink` | A ação principal **sobre bloco azul** ("Testar com fatura de exemplo", "Contestar agora") e "Copiar" |
| **Ghost** | Sem fundo, texto `accent`, espaço lateral 10 px | Ações de baixo peso ("Corrigir dados", "Por que isso foi marcado?") |
| **Contorno sobre azul** | Transparente, borda 1,5 px branca, texto branco | Ação alternativa sobre bloco azul ("Enviar minha fatura", "Já paguei esta fatura") |

Em cada área há no máximo **um** botão Primary ou Tertiary.

| Estado | Primary | Secondary | Tertiary | Ghost |
|---|---|---|---|---|
| Hover | Fundo `accent-strong` | Fundo `accent-soft` | Fundo `highlight-strong` | Fundo `accent-soft` + sublinhado |
| Focus | Anel de 3 px `accent`, afastado 2 px. Sobre azul, anel `highlight` | igual | igual | igual |
| Active | Fundo `accent-strong` + desce 1 px | Fundo `accent-soft` + desce 1 px | `highlight-strong` + desce 1 px | desce 1 px |
| Disabled | Opacidade 45%, cursor bloqueado, sem hover | igual | igual | igual |
| Loading | Spinner de 16 px + rótulo ("Aguarde"), `aria-busy="true"`, desativado, largura mantida | igual | igual | igual |

### 5.2 Cartões e superfícies

| Tipo | Aparência | Uso |
|---|---|---|
| **Cartão** | Fundo `surface`, borda 1 px `line`, raio 8 px, **sem sombra** | Achado, perguntas, lembrete, contestação |
| **Papel (fatura)** | Fundo `surface`, borda 1 px `line`, raio 8 px, `shadow-paper`. Cabeçalho com divisor tracejado. Linhas separadas por 1 px `line`. Linha de total em 700 | "Sua conta explicada" e a fatura da tela principal |
| **Bloco de cor** | Fundo `accent`, texto branco, sem raio, de ponta a ponta | Cabeçalho, abertura da tela principal, valor do resultado |
| **Citação** | Fundo `bg`, borda 1 px `line`, raio 6 px, fonte monoespaçada de 14 px | Trecho original da fatura |
| **Alerta** | Fundo `*-soft`, borda 1 px `*-line`, raio 8 px, ícone + texto | Erro, aviso do free tier, prazos, "tudo certo" |

**Linha da fatura.** Descrição à esquerda e valor à direita, em peso 600. Se o item é `indevido`, a descrição e o valor levam `marca-texto`. Abaixo (ou ao lado, quando a fatura ocupa a largura toda no desktop) vem a **anotação**: texto em `accent`, com um fio de 2 px `accent` à esquerda. A parte que diz o problema ("Você disse que não contratou.") fica em `danger` e peso 700. O selo de severidade continua presente na linha: o marca-texto não substitui o ícone + rótulo.

Não usar a faixa colorida na lateral do cartão de achado (o `EDGE` atual). A severidade é dada pelo selo.

### 5.3 Campos e formulários

- **Rótulo** sempre visível acima do campo, peso 600. Nunca usar só o placeholder.
- **Campo:** altura mínima 48 px, espaço interno 10 × 14 px, raio 6 px, fundo `surface`, borda 1 px `field-line`.
- **Focus:** anel de 3 px `accent`, afastado 2 px.
- **Erro:** borda `danger`, fundo `danger-soft`, mensagem abaixo em `danger`, peso 600, 14 px, com `aria-invalid` e `aria-describedby`. A mensagem diz como corrigir.
- **Disabled:** opacidade 60%.
- **Checkbox:** 22 × 22 px, `accent-color: accent`, rótulo em peso 600, linha inteira clicável com 44 px de altura.
- **Escolha Sim/Não** (`QuestionsPanel`): dois botões Secondary lado a lado. O escolhido vira Primary com ícone de confirmação (`aria-pressed`).
- **Dropzone:** borda tracejada de 2 px `muted`, raio 8 px, fundo `surface`, conteúdo centralizado. Ao arrastar um arquivo por cima: borda `accent` e fundo `accent-soft`.

### 5.4 Navegação

- **Navbar:** bloco `accent` de ponta a ponta, altura de 64 px. À esquerda, o logo (símbolo negativo + "HumanoBill" em branco). À direita, os links "Como funciona" e "Privacidade" (escondidos abaixo de 640 px, onde ficam só no rodapé) e o botão **A+** (44 × 44 px, contorno branco).
- **Rodapé:** fundo `bg`, divisor `line` em cima, texto `muted` de 14 px, links sublinhados.
- **Sidebar:** não existe no produto. Não criar.

### 5.5 Selos (badges) e tags

Ícone de 16 px + rótulo, peso 600, 14 px, espaço interno 4 × 10 px, raio 6 px, borda 1 px na cor do texto, fundo `*-soft`.

| Selo | Cores | Ícone |
|---|---|---|
| Indevido | `danger` sobre `danger-soft` | círculo com X |
| Suspeito, confira | `warning` sobre `warning-soft` | triângulo com exclamação |
| Tudo certo | `ok` sobre `ok-soft` | círculo com visto |

Os ícones são os que já existem em `SeverityBadge.tsx`.

### 5.6 Drawer, modal e diálogos

O `CorrectionDrawer` é o único. Painel `surface` com `shadow-lift`, raio 8 px nos cantos visíveis, fundo da página escurecido com `ink` a 50%. No celular sobe de baixo e ocupa até 90% da altura. No desktop entra pela direita com 480 px de largura. Título em H3, botão de fechar Ghost de 44 px, ações no rodapé (Primary à direita, Ghost "Cancelar"). Foco preso dentro do painel, `Esc` fecha, o foco volta ao botão que abriu.

### 5.7 Menus, dropdowns e tooltips

Não existem hoje. Se entrarem: painel `surface`, borda 1 px `line`, raio 8 px, `shadow-lift`, itens de 44 px com hover `accent-soft`. **Não usar tooltip para informação necessária**: no celular ele não aparece. Usar texto visível ou uma área que abre e fecha ("Por que isso foi marcado?").

### 5.8 Tabelas

A única tabela do produto é a fatura, que usa o padrão **Papel** (5.2). Para qualquer outra lista de dados: cabeçalho em 600 e `muted`, linhas com divisor de 1 px `line`, valores numéricos à direita, sem zebra.

### 5.9 Ícones

Traço de 2,2–2,5 px, pontas e cantos arredondados, `currentColor`, em SVG inline com `aria-hidden`. Tamanhos de 16, 20 e 24 px. **Sem emoji como ícone**: trocar 🔒 pelo cadeado em SVG e 🎉 pela ilustração `tudo-certo.svg`.

## 6. Logo e ilustrações

**Logo: "recibo grifado".** Um recibo com a borda de baixo serrilhada, duas linhas de texto e uma linha grifada de amarelo. Decisão D23.

- Símbolo para fundo claro: [`design/logo-simbolo.svg`](./design/logo-simbolo.svg). Para fundo azul: [`design/logo-simbolo-negativo.svg`](./design/logo-simbolo-negativo.svg).
- Logo completo = símbolo (28 px) + a palavra "HumanoBill" em Bricolage Grotesque 700, 20 px, `-0.02em`, com 8 px entre os dois. A palavra é texto vivo, não imagem.
- Área de respiro em volta: metade da altura do símbolo. Tamanho mínimo do símbolo: 20 px.
- O favicon (`apps/web/public/favicon.svg`) passa a ser o símbolo para fundo claro.
- Não distorcer, não trocar as cores, não aplicar sombra.

**Ilustrações.** Desenho de linha com traço `accent` de 2,5 px e preenchimentos em `highlight`, no mesmo vocabulário do logo. Sem fotos e sem banco de imagens.

| Arquivo | Onde usar |
|---|---|
| `ilustracoes/passo-envie.svg`, `passo-entenda.svg`, `passo-conteste.svg` | Os três passos de "Como funciona" (48 px) |
| `ilustracoes/caneta-marca-texto.svg` | Sobre a fatura da tela principal, no canto inferior direito |
| `ilustracoes/tudo-certo.svg` | Estado "nenhum problema encontrado" (84 px), no lugar do emoji |

Toda ilustração é decorativa (`aria-hidden="true"`): o texto ao lado já diz tudo.

## 7. Efeitos visuais e movimento

**Elevação.** Três níveis, e só eles:

| Nível | Token | Uso |
|---|---|---|
| 0 | sem sombra, borda 1 px `line` | Cartões comuns |
| 1 | `shadow-paper` | A fatura em papel |
| 2 | `shadow-lift` | Drawer e modal |

**Bordas e divisores.** 1 px `line`. Tracejado só no cabeçalho da fatura e no dropzone. Botão secundário com 1,5 px.

**Gradientes.** Nenhum, com uma exceção: o traço do marca-texto (utilitário `marca-texto`), que usa um gradiente só para dar as pontas irregulares. Remover o gradiente e o círculo decorativo do `RecoverableCard` atual.

**Movimento.**

| Onde | Como |
|---|---|
| Hover e foco | Transição de cor de 150 ms |
| Botão pressionado | Desce 1 px, 50 ms |
| Contador do valor (`CountUp`) | Mantém o atual. É o único movimento que acontece sozinho |
| Abrir/fechar (drawer, "por quê") | 200 ms, `ease-out-soft` |
| Troca de severidade do selo | Mantém a mola atual |
| Fatura da tela principal | Inclinada em −2°, parada. Sem animação |

Não animar a entrada de cada seção ao rolar. Com `prefers-reduced-motion`, o contador mostra o valor final direto e as transições somem.

**Feedback de ação.**

| Ação | Resposta |
|---|---|
| Copiar | O botão vira "Copiado!" por 2,5 s + texto "Agora é só colar" em `ok`, anunciado com `aria-live` |
| Salvar correção | Botão em Loading, depois o resultado se atualiza |
| Responder pergunta | O botão escolhido vira Primary com ícone de visto |
| Erro | Alerta `danger` no lugar onde o erro aconteceu, dizendo como resolver |
| Servidor acordando | Faixa `warning-soft` no topo, com `role="status"` |

## 8. Responsividade e acessibilidade

| Faixa | Largura | Comportamento |
|---|---|---|
| Celular | até 639 px | Uma coluna. Navbar só com logo e A+. Botões ocupam a largura toda quando estão empilhados |
| Tablet | 640–767 px | Uma coluna. Links aparecem na navbar. Botões lado a lado |
| Desktop | 768 px ou mais | Duas colunas (tela principal e resultado). Margem lateral de 40 px |
| Desktop largo | 1120 px ou mais | O conteúdo para de crescer e fica centralizado. Os blocos de cor continuam de ponta a ponta |

Breakpoints do Tailwind: `sm` (640), `md` (768), `lg` (1024). A página nunca rola na horizontal. Texto longo quebra (`break-words`) em vez de vazar.

**Acessibilidade (WCAG 2.1 AA):**
- Contraste mínimo de 4,5:1 para texto e 3:1 para bordas de campo e ícones (medidas na seção 2).
- Foco sempre visível: anel de 3 px. Sobre bloco azul, o anel é amarelo.
- Alvos de toque de 48 px (44 px para links em linha).
- Ordem dos títulos sem pular nível. Um `h1` por página.
- Estado nunca só por cor: ícone + texto.
- `lang="pt-BR"`, link "Ir para o conteúdo", `aria-live` em resultados que mudam.
- Nada essencial em hover.

**Modo idoso (D18):**

| Recurso | Comportamento |
|---|---|
| **A+** (RF20) | Botão fixo na navbar. Sobe o tamanho base para 125% (texto de ~21 px, botões de 56 px). Preferência salva no `localStorage` |
| **Ouvir** (RF21) | Botão Ghost com ícone de alto-falante no resumo e em cada achado. `speechSynthesis` em pt-BR. Escondido se o navegador não suportar. *Hoje existe só o botão do resumo, que lê o total, o valor indevido e cada achado; o botão por achado ficou para depois* |
| **Modo passo a passo** (RF22) | Opção na tela principal. Uma etapa por tela, botões "Próximo"/"Voltar" grandes, sem termos técnicos |

## 9. Telas

### 9.1 Tela principal `/`
- **Navbar** azul (5.4).
- **Bloco de abertura** azul de ponta a ponta. À esquerda: título em Display **"Sua conta, em português claro."**, subtítulo "Descubra em segundos se você está pagando o que não deve, e conteste com o texto pronto.", botão Tertiary **Testar com fatura de exemplo**, botão de contorno **Enviar minha fatura**, e a linha "Seus dados pessoais não são guardados." com o cadeado em SVG. À direita (embaixo no celular): uma fatura em papel inclinada em −2°, com duas linhas grifadas, duas anotações e a caneta marca-texto por cima.
- **Escolha do exemplo:** abre abaixo do bloco, em cartão, com as faturas agrupadas (telecom, energia) como botões Secondary.
- **Como funciona:** três células lado a lado no desktop, separadas por divisores de 1 px, cada uma com ilustração, "1. Envie", "2. Entenda", "3. Conteste" e uma frase. Empilhadas no celular.
- Dispara o warm-up (`GET /health`) ao carregar.

### 9.2 Enviar `/analisar`
- Título em H1 sobre fundo papel (sem bloco azul).
- Dropzone (5.3) com "Escolher arquivo" (Primary) e "Tirar foto" (Secondary).
- Aviso do free tier em alerta `warning` + checkbox de consentimento (obrigatório).
- Botão **Analisar** (Primary, largura toda).
- Progresso em etapas: *Lendo sua fatura…* → *Conferindo item por item…* → *Pronto*.

### 9.3 Resultado `/r/:id`
- **Bloco do valor** azul de ponta a ponta, logo abaixo da navbar: rótulo "Parece indevido" (ou "Valor recuperável"), o valor em `text-hero` com `marca-texto`, a frase de apoio ("em 2 cobranças desta fatura de R$ 122,70"), botão Tertiary **Contestar agora** e botão de contorno **Já paguei esta fatura**. Operadora e mês aparecem acima do rótulo, em 14 px.
- **Perguntas rápidas** (se houver), em cartão, logo abaixo do bloco.
- **Duas colunas no desktop:** à esquerda os **achados** (cartões com selo, valor, título, explicação, trecho original, base legal e a caixa "Incluir na contestação"); à direita **"Sua conta explicada"**, a fatura em papel com as linhas grifadas e as anotações. No celular: achados primeiro, fatura depois.
- **Energia:** gráfico do histórico de 12 meses em `accent`, com o mês atual em `highlight` com contorno `ink`.
- Link Ghost "Os dados estão errados? Corrigir" abre o drawer.
- **Contestação**, lembrete e comparação seguem abaixo, em largura total, com texto limitado a 65ch.
- Rodapé: "Este resultado expira em 30 dias" + copiar link.

### 9.4 Conferir dados (drawer)
Campos extraídos editáveis (5.3 e 5.6). Salvar reavalia na hora, sem IA.

### 9.5 Contestação
- Texto gerado em um bloco de citação grande (fonte do texto, não monoespaçada) com botão **Copiar** (Tertiary).
- Passo a passo numerado com links (SAC → Anatel/ANEEL → consumidor.gov.br) e os prazos em alerta `warning`.
- Aviso: "conteúdo informativo, não é orientação jurídica".
- **Roteiro de ligação (RF17):** alternador "Prefiro ligar", roteiro numerado em H4 e o campo "anote o protocolo aqui".
- **Lembrete (RF18/RF19):** cartão com campo de e-mail, checkbox de consentimento separado e a frase "seu e-mail é apagado depois do aviso".
- **Comparar faturas (RF16):** itens **novos**, **removidos** e **alterados** + variação do total.
- **Composição da conta (RF23):** barra empilhada em tons de `accent`, com **extras** em `highlight`.

### 9.6 Privacidade `/privacidade`
Uma página curta e honesta, em texto de até 65ch: o que é processado, por quem, por quanto tempo e o que nunca guardamos.

## 10. Estados obrigatórios

| Estado | Comportamento |
|---|---|
| Carregando a API (cold start) | Skeleton em `line` + faixa "acordando o servidor…" depois de 2 s |
| Arquivo inválido / grande demais | Mensagem em `danger` dentro do dropzone |
| Extração falhou | Alerta `danger`: "Não consegui ler bem essa fatura. Tente um PDF ou uma foto mais nítida", com um botão para usar o exemplo |
| Nenhum problema encontrado | Alerta `ok` com a ilustração `tudo-certo.svg` e "Tudo certo com esta conta" + itens explicados |
| Resultado expirado | 410 → "Este resultado expirou. Analise de novo." |
| Rate limit | "Muitas análises seguidas, tente em alguns minutos" |

## 11. Copy: tom de voz

- **Sim:** "Isso parece um serviço extra que você pode não ter contratado."
- **Não:** "Foi identificada irregularidade na cobrança do item supramencionado."
- **Nunca:** "Você vai ganhar R$ X", "garantido", "advogado".
- Botão diz o que acontece: "Copiar", "Analisar", "Contestar agora". Erro diz como resolver.

## 12. Guia de implementação

Esta seção é para quem vai aplicar o guia em `apps/web`.

**O que não pode mudar:** regras de negócio, rotas, chamadas de API, `src/lib/`, `src/types/` e o comportamento das telas. A mudança é só visual e de estrutura de layout. Os textos conferidos pelos testes (`src/pages/*.test.tsx`, `FindingCard.test.tsx`) continuam iguais, salvo a frase de apresentação.

**Ordem sugerida** (um commit pequeno por passo):

1. **Fontes e tokens.** Auto-hospedar Bricolage Grotesque e Atkinson Hyperlegible Next. Trocar o `@theme` e o `@layer base` de `src/index.css` pelo conteúdo de `docs/design/tokens.css`. Como os nomes dos tokens foram mantidos, boa parte do app já muda de cor aqui.
2. **Logo e favicon.** Copiar o símbolo para `public/favicon.svg`. Criar o componente de logo. Copiar as ilustrações para `src/assets/` (ou transformar em componentes).
3. **Componentes base** em `src/components/ui.tsx`: `Button` e `ButtonLink` com as variantes `primary`, `secondary`, `tertiary`, `ghost` e `onDark`, mais a prop `loading`; `Card` sem sombra; novos `Paper`, `Alert` e `Field`. Trocar os botões escritos à mão em `RecoverableCard`, `FindingCard` e `QuestionsPanel` por esses componentes.
4. **Layout** (`src/components/Layout.tsx`): navbar azul com logo, links e A+; tirar o `max-w-3xl` do `<main>` para permitir blocos de ponta a ponta; rodapé.
5. **Tela principal** (`src/pages/Landing.tsx`): bloco de abertura, fatura ilustrada, três passos com ilustração (seção 9.1).
6. **Resultado:** `RecoverableCard` vira o bloco do valor; `BillItems` vira a fatura em papel com marca-texto e anotações; `FindingCard` perde a faixa lateral; `ResultBody` ganha as duas colunas; `AllClear` usa a ilustração (seção 9.3).
7. **Demais telas:** `Analyze.tsx`, `DisputeSection`, `QuestionsPanel`, `CorrectionDrawer`, `EnergyPanel`, `Privacy.tsx`.
8. **Limpeza de cores.** Trocar todas as classes cruas do Tailwind por tokens. Hoje são 33, em `Analyze.tsx`, `Landing.tsx`, `Layout.tsx`, `ui.tsx` e `Privacy.tsx`. Para conferir: `grep -rnE '(slate|emerald|amber|red|green|gray)-[0-9]+' apps/web/src` deve voltar vazio.
9. **Frase de apresentação.** Trocar "português humano" por "português claro" em: `apps/web/index.html` (`<title>`), `apps/web/src/pages/Landing.tsx`, `README.md`, `docs/README.md`, `docs/08-demo-video-pitch.md` e `docs/11-submissao-e-pitch.md`. O nome **HumanoBill** não muda.
10. **Screenshots.** Refazer as imagens do README com o visual novo.

**Verificação:**
- Em `apps/web`: `npm run test`, `npm run lint` e `npm run build` sem erro.
- Percorrer tela principal → exemplo → resultado → contestação em 375, 768 e 1280 px, comparando com `docs/design/referencia.html`.
- Navegar só com o teclado: o foco aparece em tudo, inclusive sobre os blocos azuis.
- Ligar o A+ e conferir que nada quebra ou vaza.
- Ligar `prefers-reduced-motion` e conferir que o contador não anima.
- Auditoria de acessibilidade do Lighthouse sem falha de contraste.
- Prazo: tudo commitado antes de **30/11 às 23h59**. Depois disso o repositório fica congelado.
