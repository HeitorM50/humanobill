# 08 — Demo, vídeo e pitch

## Faturas sintéticas (item 6.9: nenhum dado real)

Marcas **fictícias** com layout realista. Clientes, endereços e números são todos inventados (ex.: "Maria Exemplo da Silva", CPF de teste inválido).

| Fixture | Marca | Cenário | Achados esperados |
|---|---|---|---|
| `telecom-1` | Vivaz Telecom (celular) | 2 SVAs escondidos ("Clube Vivaz Música" R$ 12,90, "Seguro Tela" R$ 9,90) | TEL-01 ×2 (`suspeito` → `indevido` ao responder "não contratei") |
| `telecom-2` | Vivaz Fibra (internet + TV) | Fatura paga com atraso: multa de 5% + juros de 3% ao mês; mensalidade acima do plano | TEL-03, TEL-04 (`indevido`), TEL-05 (`suspeito`) |
| `telecom-3` | Vivaz Telecom | Conta limpa | Nenhum achado. Testa o estado "tudo certo" e garante zero falso positivo |
| `energia-1` | Luzia Energia | Consumo 62% acima da média de 12 meses + leitura estimada | ENE-02, ENE-03 (`suspeito`) |
| `energia-2` | Luzia Energia | "Seguro Residencial Luzia" embutido + multa acima de 2% | ENE-07, ENE-01 |
| `energia-3` | Luzia Energia | Bandeira vermelha cobrada num mês de bandeira verde (tabela fixture) | ENE-05 |

**Como gerar:** um template HTML/CSS por marca → PDF (via navegador headless ou print). Gerar também uma versão "foto" (PNG levemente inclinado) para mostrar que a leitura funciona com imagem. Os templates ficam em `fixtures/templates/` e são versionados.

## Roteiro do vídeo (≤ 3:00, em português)

### O que o regulamento exige (item 7.2)

Até 3 minutos, link que abre **sem login** e o **produto funcionando**. Português ou inglês (9.8). O vídeo é a nota de **comunicação** da fase online e, se a demo cair, o júri avalia só pelo vídeo e pelo código (7.3). Plataforma, narração, legenda e formato são livres.

### Princípios usados (pesquisa de 26/11/2026)

1. **Abrir com uma pessoa, uma dor e um custo**, em 15 a 20 s. Nada de "nós somos".
2. **Uma frase de solução** que um leigo repete depois de ouvir uma vez (teste do Michael Seibel, YC).
3. **Pelo menos metade do tempo é produto funcionando**, de verdade, não slide.
4. **Um momento "uau"**, e não um tour por todas as features: aqui é a pergunta "Você contratou?" virando "indevido" e o valor em dobro.
5. **Uma ideia central** que o júri leva para casa: *a IA lê, a regra decide, com a lei na mão.*
6. **Um insight não óbvio:** quem acusa não pode ser a IA, porque falso positivo grave custa caro (C03 e C06).
7. **Fechar com impacto e próximo passo**, e repetir a frase da marca.
8. Narrativa clara vale mais que edição cara. Gravar cedo, porque a edição sempre demora mais.

### Roteiro (cerca de 350 palavras: ~2:30 de fala a 140 palavras por minuto, com ~30 s de respiro para as telas)

Toda a demo é gravada em tela de celular (390 px no modo dispositivo do navegador ou o celular de verdade), com os dados fictícios das fixtures.

| Tempo | Cena (o que aparece) | Narração | Texto na tela |
|---|---|---|---|
| 0:00–0:15 | **Gancho.** A imagem da fatura `telecom-1` (Vivaz Telecom). Zoom lento até as linhas "Clube Vivaz Música R$ 12,90" e "Seguro Tela R$ 9,90", que ganham o grifo amarelo | "Essa é a conta de celular da dona Maria: cento e vinte e dois reais e setenta. No meio dela, vinte e dois e oitenta de serviços que ela nunca pediu. Ela não percebeu. Quase ninguém percebe." | "R$ 22,80 que ela não pediu" |
| 0:15–0:30 | **Problema em número.** Fundo azul, número grande | "Cobrar por serviço não contratado é a reclamação de consumo número um do Brasil: duzentos e oitenta e seis mil casos só em 2025. A lei manda devolver em dobro. Mas a fatura é ilegível, e reclamar dá trabalho." | "286 mil casos · nº 1 do país" + "Fonte: Senacon, 2025" |
| 0:30–0:40 | **Solução.** Landing do HumanoBill | "O HumanoBill lê a sua fatura, explica cada linha em português claro, aponta o que está errado com a lei na mão e entrega a contestação pronta." | "Sua conta, em português claro." |
| 0:40–1:00 | **Envio real.** Botão "Analisar com IA de verdade" (ou upload da foto) → tela de progresso → resultado. Cortar a espera na edição | "Eu envio a foto da conta. A IA lê a fatura, sem guardar nome nem CPF. Em segundos: vinte e dois e oitenta para conferir, marcados na própria fatura." | "Leitura real pela IA · ~9 s" |
| 1:00–1:35 | **Momento uau.** Achado do Clube Vivaz Música com o trecho original e "RGC Anatel, art. 48". Pergunta "Você contratou Clube Vivaz Música?" → **Não** → o selo muda de "Suspeito" para "Indevido". Depois "Já paguei" → o valor recuperável vira **R$ 25,80** | "Cada achado mostra o trecho original e o artigo da lei. Aqui, o regulamento da Anatel: serviço extra só com autorização. Mas o app não acusa sozinho. Ele pergunta: você contratou? Não. Agora sim: indevido. E como a conta já foi paga, a devolução é em dobro." | "Suspeito → Indevido" · "Em dobro: R$ 25,80" |
| 1:35–2:00 | **Contestação.** "Gerar contestação" → texto pronto → "Copiar" → passo a passo (SAC → Anatel → consumidor.gov.br) → "Baixar carta (PDF)" → Minhas contas com o prazo contando | "Um toque e a contestação está pronta, com os artigos citados. É só colar no chat da operadora ou baixar a carta em PDF. O app guarda o protocolo, conta os 30 dias da operadora e, se ela não responder, mostra o próximo passo." | "Prazo da operadora: 30 dias" |
| 2:00–2:20 | **Mais contas e acessibilidade** (cortes rápidos de 3 s): `energia-1` com o gráfico 62% acima da média e "Leitura estimada" → toque em **TUSD** abrindo o balão → extrato e cartão → botão **A+** → modo passo a passo → simulador de WhatsApp | "Funciona também com conta de luz, extrato do banco e fatura do cartão. Não sabe o que é TUSD? Um toque explica. E para quem tem mais dificuldade: letra maior, leitura em voz alta, uma pergunta por tela. E até pelo WhatsApp." | "Luz · banco · cartão · WhatsApp" |
| 2:20–2:42 | **Como funciona.** Diagrama "a IA lê, a regra decide" (o do README) → terminal com os testes passando | "O segredo está na divisão. A IA só lê. Quem decide é um motor de regras em Rust, sem IA, que cita a lei e tem mais de mil e duzentos testes. Se a base legal não foi conferida, ele nunca diz indevido. E o arquivo não é salvo: o resultado some em 30 dias." | "A IA lê. A regra decide." · "1.218 testes" |
| 2:42–3:00 | **Fechamento.** Painel coletivo → logo + frase + endereço | "Cada análise também alimenta um painel anônimo das cobranças indevidas mais comuns, um dado que pode virar pressão no Procon e na Anatel. HumanoBill: sua conta, em português claro." | "humanobill.vercel.app" |

**Se passar de 3:00, cortar nesta ordem:** o simulador de WhatsApp na cena 2:00 → a frase do painel no fechamento (fica só a marca) → o extrato e o cartão. Nunca cortar o momento uau nem a contestação.

### Números citados (conferidos em 26/11 na API de produção)

- `telecom-1`: total R$ 122,70; Clube Vivaz Música R$ 12,90 + Seguro Tela R$ 9,90 = R$ 22,80.
- Responder "não" ao Clube Vivaz Música + "já paguei": TEL-01 vira `indevido` e o recuperável é R$ 25,80 (2 × R$ 12,90).
- `energia-1`: 324 kWh, 62% acima da média de 200 kWh, leitura estimada.
- Testes: 874 em Rust + 344 no front = 1.218. Atualizar se mudar até a gravação.
- 286 mil casos e 28,7% das reclamações em 2025: [boletim ProConsumidor da Senacon](https://www.gov.br/mj/pt-br/assuntos/seus-direitos/consumidor/defesadoconsumidor/Biblioteca/boletins-1/boletim_proconsumidor_sindec__2025_v8.pdf).
- O painel coletivo ainda tem poucas faturas: mostrar a tela, **sem citar porcentagem** como dado do país.

### Produção

- **Narração:** gravar a própria voz primeiro (microfone de fone num quarto com roupas já resolve). Se usar voz de IA, escolher uma voz pt-BR natural, testar a pronúncia de "TUSD", "Anatel", "R$ 25,80" e "art. 48" e ajustar a escrita das falas para o ouvido. Nunca clonar a voz de outra pessoa.
- **Tela:** 1080p, modo celular, cursor ou toque visível, sem notificações. Gravar cada cena separada para editar com folga.
- **Legenda em PT** queimada no vídeo (muita gente assiste sem som).
- **Música:** só livre de direitos e baixa, por baixo da voz.
- **Publicar:** YouTube **não listado**, testar o link em aba anônima e conferir a duração (≤ 3:00).
- **Ensaio:** mostrar para alguém de fora e pedir que repita o que o app faz. Se não conseguir, a frase da solução precisa mudar.

## Pitch na final (12/12)

- O formato chega por e-mail com a lista de finalistas (item 8.3). Adaptar a duração depois.
- Estrutura: dor (com número) → demo ao vivo (com plano B: vídeo gravado) → como funciona (IA + regras) → impacto → roadmap/negócio → pedido.
- Levar a demo rodando **localmente também** (caso a rede do local falhe).

## Checklist da submissão (item 7.2): conferir em 29/11

- [ ] Nome: **HumanoBill**
- [ ] Frase de apresentação: *"Sua conta, em português claro: descubra cobranças indevidas na fatura de telecom e luz e conteste com o texto pronto."*
- [ ] Descrição (problema, solução, como funciona, tecnologias) — texto pronto em [11 — Submissão e pitch](./11-submissao-e-pitch.md)
- [ ] Trilha: **Pagamentos & finanças**
- [ ] Link do repo GitHub (público, criado depois de 04/11 às 21h, MIT)
- [ ] Link da demo (Vercel) testado em aba anônima e no celular
- [ ] Link do vídeo ≤ 3 min, abre sem login
- [ ] Aceite do regulamento no painel
- [ ] README do repo com screenshots, arquitetura e "como rodar"
- [ ] Keep-alive ativo e alerta do UptimeRobot configurado
- [ ] Último commit antes de 30/11 às 23h59
