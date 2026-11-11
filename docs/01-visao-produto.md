# 01 — Visão de produto

## O problema

As faturas de telecom e de energia são difíceis de ler. Nelas aparecem itens que a pessoa não contratou, encargos acima do permitido e cobranças duplicadas. Quase ninguém confere, e quem percebe desiste por causa da burocracia da contestação.

**Os dados (pesquisa de 10/11/2026):**
- Cobrança por serviço não contratado é o **principal problema de consumo do país em 2025**, com 286.265 casos (28,67%) no ProConsumidor/Sindec ([boletim Senacon 2025](https://www.gov.br/mj/pt-br/assuntos/seus-direitos/consumidor/defesadoconsumidor/Biblioteca/boletins-1/boletim_proconsumidor_sindec__2025_v8.pdf)).
- Telecom (29,8%) e serviços financeiros (25,6%) são os setores mais reclamados no consumidor.gov.br entre 2015 e 2022 ([estudo UEPG](https://revistas.uepg.br/index.php/admpg/article/download/23945/209209219703/209209268936)).
- Em telecom, a cobrança indevida responde por cerca de 28% das reclamações ([Idec](https://idec.org.br/node/28542)).
- Os **SVAs** (serviços de valor adicionado) são uma fonte clássica de cobrança escondida na conta de celular e de internet ([Idec](https://idec.org.br/dicas-e-direitos/cobranca-indevida-o-que-sao-os-svas-na-conta-de-internet-e-telefone)).
- Na conta de luz, o Idec cobra da ANEEL mais transparência sobre erros de cobrança ([Idec](https://idec.org.br/em-acao/em-foco/aneel-e-cobrada-por-maior-transparencia-quanto-ao-erro-na-conta-de-luz)).

**As regras favorecem o consumidor, mas ninguém as usa:**
- **CDC, art. 42, parágrafo único:** quem paga uma cobrança indevida tem direito à devolução em dobro. Os tribunais têm dispensado a prova de má-fé ([Idec](https://idec.org.br/consultas/dicas-e-direitos/devoluco-em-dobro-veja-quando-o-consumidor-tem-esse-direito), [Âmbito Jurídico](https://ambitojuridico.com.br/reembolso-em-casos-de-cobranca-indevida/)).
- **Anatel (RGC):** o consumidor pode contestar em até 3 anos, e a operadora tem 30 dias para responder. Sem resposta, ela precisa corrigir a fatura ou devolver em dobro, com juros de 1% ao mês ([Idec](https://idec.org.br/em-acao/em-foco/comeca-a-valer-hoje-parte-do-novo-regulamento-geral-de-direitos-do-consumidor-de-telecomunicaces)).
- O **número de protocolo** é a prova de que a empresa foi notificada ([Anatel](https://sistemas.anatel.gov.br/anexar-api/publico/anexos/download/a06d1e2620d4a0942824b5a430b4d9d1)).

## Público

**Principal:** pessoa física adulta que paga as próprias contas de celular, internet, TV e luz. Não tem tempo nem conhecimento para conferir item por item.

**Personas da demo:**
- **Dona Cida, 58:** paga a conta de celular há anos e nunca reparou num "Pacote Seguro Digital" de R$ 9,90 por mês.
- **Wagner, 34, motorista de app:** a conta de luz veio 40% acima da média sem nenhuma mudança de hábito.

## Proposta de valor

1. **Entender:** cada linha da fatura explicada em uma frase simples.
2. **Detectar:** problemas apontados por regras determinísticas, cada um com a base legal.
3. **Recuperar:** quanto dá para pedir de volta (o dobro, se já foi pago).
4. **Agir:** contestação pronta para copiar, com o passo a passo (SAC → agência reguladora → consumidor.gov).

## Posicionamento: o que o HumanoBill é e o que não é

- **É:** uma ferramenta de **informação e organização** do consumidor. Explica, aponta indícios e prepara o texto.
- **Não é:** advogado, nem promessa de resultado. Também não contesta em nome do usuário.
- Cada achado é rotulado como `indevido` (regra objetiva violada), `suspeito` (vale conferir) ou `ok`.
- **Lição do DoNotPay:** o "robot lawyer" americano sofreu sanção da FTC por propaganda enganosa. Nada de linguagem de "advogado robô".

## Concorrência

| Quem | O que faz | Por que não resolve |
|---|---|---|
| CSU Digital (HAS) | Hiperautomação com IA para contestação de fatura **do lado da operadora** ([Mobile Time](https://www.mobiletime.com.br/noticias/23/09/2024/hiperautomacao/)) | É B2B e serve a operadora |
| Algar (Billy) | IA + RPA para revisão de faturas, economizou R$ 3 mi ([DPL News](https://dplnews.com/?p=265094)) | É B2B e uso interno |
| Unicainstancia | Indenização por cobrança indevida em telefonia ([Startups.com.br](https://startups.com.br/negocios/unicainstancia-cresce-ao-ajudar-consumidor-com-indenizacoes-por-cobrancas-indevidas/)) | Modelo jurídico/indenizatório, sem explicar a conta. Matéria de 2022 |
| Meu Plano (2018) | Upload de PDF da conta telefônica com orientações ([Baguete](https://www.baguete.com.br/noticias/app-confere-a-conta-telefonica)) | Antigo, sem IA moderna, status desconhecido |
| DoNotPay (EUA) | Chatbot que negocia contas e contesta cobranças ([Scripps](https://scrippsnews.com/stories/chatbot-can-haggle-with-cable-company-to-lower-bill/)) | Não atua no Brasil e não conhece a regulação local |

**Gap:** não achamos nenhum produto B2C brasileiro que junte explicação da fatura, detecção com base legal e contestação pronta, nem que cubra telecom **e** energia.

## Riscos de flop e mitigação

| Risco | Por que mataria o projeto | Mitigação |
|---|---|---|
| Ser visto como "wrapper de LLM" | Derruba a nota de implementação técnica | A IA só extrai e explica. Quem decide é o motor de regras em Rust, sem I/O e testado |
| Falso positivo | Acusar cobrança legítima destrói a confiança | 3 níveis de severidade. `indevido` só sai com regra objetiva. Sempre mostrar o porquê |
| Erro de extração da IA | Regra certa sobre dado errado dá conclusão errada | Validação do JSON (schema + checagem de soma dos itens = total). Tela de "confira os dados" editável |
| Regras de energia nebulosas | Conclusões frágeis na conta de luz | Regras de energia mais conservadoras (`suspeito` por padrão), marcadas "a validar" até ter fonte oficial |
| Fatura real na demo | Viola o item 6.9 (dados reais) | Faturas sintéticas com marcas fictícias. Aviso e consentimento para upload real |
| Escopo grande para 1 pessoa | Produto incompleto derruba a execução técnica | Roadmap com cortes definidos. O fluxo principal tem prioridade sobre features |

## Enquadramento na trilha (itens 5.4 e 5.5)

**Trilha: Pagamentos & finanças.** O exemplo oficial da trilha inclui "gestão financeira pessoal".
- **O problema** (pagar o que não deve) e **o público** (pessoa física e o próprio orçamento) estão dentro da trilha.
- O resultado central é **dinheiro**: quanto você paga a mais e quanto pode recuperar.
- **Não** é apostas, cripto nem promessa de rendimento (item 6.8).

## Contexto do hackathon (pesquisa de 11/11/2026)

- **1ª edição:** não encontramos edições anteriores do Hack Brasil nem ganhadores passados. O [site oficial](https://hackbrasil.com) e a imprensa ([Brasil Inovador](https://brasilinovador.com.br/hack-brasil-2026-distribui-r-12-milhao-e-tera-fase-decisiva-na-sede-do-google-brasil/), [Inteligência Móvel](https://www.inteligenciamovel.com.br/hackathon-com-final-no-google-brasil/)) só tratam da edição 2026.
- **O que o júri valoriza (imprensa):** "profissionais de mercado" avaliando **viabilidade econômica**: protótipos **escaláveis, seguros e usáveis, com potencial comercial**, maduros para investimento anjo ou aceleração.
- **Histórico dos organizadores:** os hackathons anteriores da Vibe Space Academy ([Cognition Hack SP](https://luma.com/cognition-hack-sp), [Zero to Agent](https://luma.com/02ASP)) usavam como critérios **qualidade técnica, apresentação visual, impacto e originalidade**, e os 5 melhores faziam pitch de 3 min.
- **Concorrência visível no GitHub (11/11):** cerca de 25 repositórios públicos de outros times, e **nenhum** analisador de faturas. Na mesma trilha: planejamento para sair de dívidas e radar da reforma tributária para contadores.

**Ajustes no plano por causa disso:**
1. A **viabilidade comercial** (mercado, modelo de receita, canais, riscos) vira item de pitch e de README (issue #64, milestone Final).
2. O **polish visual** fica protegido: as animações são o último item da ordem de corte.
3. **Segurança e usabilidade** já são Must (RNF06–RNF09).

## Métricas de sucesso

- **No hackathon:** ir à final (≥ 20/40 e top 5 da trilha) e ficar no top 3 na final.
- **No produto (demo):** analisar uma fatura em menos de 15 s, com 100% dos achados das faturas sintéticas detectados e zero falso `indevido`.
