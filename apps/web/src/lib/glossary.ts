/**
 * Glossário das faturas (RF24). Texto em português claro, sem juridiquês.
 * Regra da casa: nada de valor em reais por kWh (eles mudam) e nenhum número ou artigo
 * além dos que já estão nas regras do app.
 */

export type GlossaryTerm = {
  /** Slug usado na âncora: `/glossario#tusd`. */
  id: string
  term: string
  /** Como o termo aparece escrito nas faturas. Servem para achar o termo na descrição de um item. */
  aliases: string[]
  /** Uma frase, para o balão. */
  short: string
  /** De duas a quatro frases simples, para a página. */
  long: string
}

export const GLOSSARY: GlossaryTerm[] = [
  {
    id: 'te',
    term: 'TE (Tarifa de Energia)',
    aliases: ['TE', 'Tarifa de Energia'],
    short: 'O preço da energia em si, a que você gastou em casa.',
    long: 'Paga a geração da energia que chega até você. É calculada pelos kWh que você consumiu. Os fios e os postes que levam essa energia até a sua casa ficam em outra linha, a TUSD.',
  },
  {
    id: 'tusd',
    term: 'TUSD (Tarifa de Uso do Sistema de Distribuição)',
    aliases: ['TUSD', 'Tarifa de Uso do Sistema de Distribuição'],
    short: 'O custo de levar a energia até a sua casa.',
    long: 'Paga o uso dos fios, dos postes e dos transformadores da distribuidora, e a manutenção da rede. Também é calculada pelos kWh que você consumiu. Costuma vir numa linha separada da TE.',
  },
  {
    id: 'cip',
    term: 'CIP ou COSIP (Contribuição de Iluminação Pública)',
    aliases: ['Contribuição de Iluminação Pública', 'Iluminação Pública', 'COSIP', 'CIP'],
    short: 'Taxa da prefeitura para pagar a luz das ruas e das praças.',
    long: 'Aparece na conta de luz, mas quem define o valor é a prefeitura da sua cidade. A distribuidora só faz a cobrança. Se tiver dúvida sobre o valor, a conversa é com a prefeitura.',
  },
  {
    id: 'icms',
    term: 'ICMS',
    aliases: ['ICMS'],
    short: 'Imposto do estado, cobrado sobre a energia e sobre serviços como o de telefone.',
    long: 'É o Imposto sobre Circulação de Mercadorias e Serviços. Cada estado define a porcentagem. Em algumas faturas ele já vem dentro do preço, em outras aparece numa linha separada.',
  },
  {
    id: 'pis-cofins',
    term: 'PIS/COFINS',
    aliases: ['PIS/COFINS', 'PIS e COFINS', 'COFINS', 'PIS'],
    short: 'Dois tributos federais que a empresa repassa para a sua conta.',
    long: 'A empresa inclui esses tributos no preço do serviço. A porcentagem pode mudar de um mês para o outro. Por isso eles aparecem como uma linha de impostos, e o valor não é sempre igual.',
  },
  {
    id: 'sva',
    term: 'SVA (Serviço de Valor Adicionado)',
    aliases: ['Serviço de Valor Adicionado', 'SVA'],
    short: 'Um serviço extra, cobrado à parte do seu plano, como clube de música ou antivírus.',
    long: 'Só pode ser cobrado com autorização prévia e expressa de quem usa (RGC, art. 48). Se você não pediu, pode contestar. O HumanoBill marca esses itens para você conferir.',
  },
  {
    id: 'bandeira',
    term: 'Bandeira tarifária',
    aliases: ['Bandeira Tarifária', 'Bandeira Verde', 'Bandeira Amarela', 'Bandeira Vermelha', 'Bandeira'],
    short: 'A cor do mês que mostra se gerar energia está mais cara. Se está, a conta leva um acréscimo por kWh.',
    long: 'Verde: sem acréscimo. Amarela: acréscimo por kWh, porque gerar energia ficou mais caro. Vermelha, patamar 1 e patamar 2: acréscimo por kWh maior ainda, por exemplo quando chove pouco. A ANEEL decide a bandeira de cada mês.',
  },
  {
    id: 'leitura-estimada',
    term: 'Leitura estimada (faturamento pela média)',
    aliases: ['Leitura estimada', 'Faturamento pela média', 'Consumo estimado'],
    short: 'Ninguém leu o medidor, e a conta foi calculada pela média dos meses anteriores.',
    long: 'A distribuidora só pode cobrar assim em alguns casos, como medidor sem acesso ou com defeito. Se acontece mês após mês, vale perguntar o motivo. O HumanoBill avisa quando a leitura da sua fatura não foi real.',
  },
  {
    id: 'kwh',
    term: 'kWh (quilowatt-hora)',
    aliases: ['kWh', 'Quilowatt-hora'],
    short: 'A unidade que mede a energia que você gastou.',
    long: 'Um aparelho de 1.000 watts ligado por uma hora gasta 1 kWh. A conta multiplica os kWh que você consumiu pelo preço de cada kWh. Quanto mais aparelhos ligados por mais tempo, mais kWh.',
  },
  {
    id: 'multa-juros',
    term: 'Multa e juros por atraso',
    aliases: ['Multa por atraso', 'Juros por atraso', 'Juros de mora', 'Multa moratória', 'Multa', 'Juros'],
    short: 'Cobrados só quando a conta é paga depois do vencimento.',
    long: 'A multa é um valor cobrado uma vez pelo atraso. Os juros crescem conforme os dias que passam. Os dois só podem aparecer se houve atraso, e a lei limita quanto se pode cobrar. Se você pagou em dia e veio multa, confira.',
  },
  {
    id: 'tarifa-essenciais',
    term: 'Tarifa e pacote de serviços essenciais',
    aliases: ['Pacote de serviços essenciais', 'Serviços essenciais', 'Pacote de serviços', 'Tarifa de serviços', 'Tarifa bancária'],
    short: 'Cobrança do banco por serviços da conta. Os essenciais têm uma cota grátis por mês.',
    long: 'Para quem é pessoa física, serviços essenciais como saques, extratos e folhas de cheque têm uma cota por mês sem cobrança. Só o que passa da cota, ou um serviço fora dessa lista, pode ter tarifa. Olhe o nome da tarifa no extrato e o que ela cobre.',
  },
  {
    id: 'anuidade',
    term: 'Anuidade',
    aliases: ['Anuidade'],
    short: 'O valor que o cartão cobra por você poder usá-lo, em geral dividido em parcelas mensais.',
    long: 'Alguns cartões têm anuidade, outros não. Se o cartão foi oferecido sem anuidade, ela não deveria aparecer na fatura. Confira também se uma isenção combinada está valendo.',
  },
]

/* ---------- Busca ---------- */

/**
 * Tira acento e põe em minúsculas, mantendo o mesmo tamanho do texto. É isso que deixa
 * as posições achadas no texto "dobrado" valerem para o texto original.
 */
function fold(s: string): string {
  return s
    .normalize('NFC')
    .split('')
    .map((c) => c.normalize('NFD')[0])
    .join('')
    .toLowerCase()
}

/** Termos do glossário cujo texto contém todas as palavras buscadas. Quem casa no nome ou no apelido vem antes. */
export function searchGlossary(q: string): GlossaryTerm[] {
  const words = fold(q).split(/\s+/).filter(Boolean)
  if (words.length === 0) return GLOSSARY
  const hits: { term: GlossaryTerm; score: number }[] = []
  for (const term of GLOSSARY) {
    const names = fold([term.term, ...term.aliases].join(' '))
    const all = `${names} ${fold(`${term.short} ${term.long}`)}`
    if (!words.every((w) => all.includes(w))) continue
    hits.push({ term, score: words.every((w) => names.includes(w)) ? 0 : 1 })
  }
  // `sort` é estável: dentro de cada grupo vale a ordem da lista.
  return hits.sort((a, b) => a.score - b.score).map((h) => h.term)
}

/* ---------- Termos dentro de um texto ---------- */

export type GlossaryMatch = {
  term: GlossaryTerm
  /** Trecho como está no texto original (com acento e caixa). */
  text: string
  /** Posição no texto original: `text.slice(start, end)`. */
  start: number
  end: number
}

const isLetter = (c: string | undefined) => c !== undefined && /[a-z]/.test(c)

/** Siglas curtas (TE, PIS, CIP, SVA) só valem em maiúsculas: "te" é uma palavra comum. */
const isShortAcronym = (alias: string) => alias.length <= 3 && alias === alias.toUpperCase()

type Candidate = { term: GlossaryTerm; folded: string; raw: string; caseSensitive: boolean }

const CANDIDATES: Candidate[] = GLOSSARY.flatMap((term) =>
  term.aliases.map((raw) => ({ term, raw, folded: fold(raw), caseSensitive: isShortAcronym(raw) })),
  // O mais longo primeiro: "Bandeira tarifária" ganha de "Bandeira".
).sort((a, b) => b.folded.length - a.folded.length)

/**
 * Acha os termos do glossário numa descrição de item, em ordem de aparição.
 * Só casa palavra inteira (TE não casa em "TELEFONE" nem em "Telecom"), ignora acento e caixa,
 * e devolve cada termo uma vez só (a primeira ocorrência).
 */
export function findTermsIn(text: string): GlossaryMatch[] {
  const folded = fold(text)
  const taken: GlossaryMatch[] = []
  for (const c of CANDIDATES) {
    if (taken.some((m) => m.term.id === c.term.id)) continue
    let from = 0
    for (;;) {
      const start = folded.indexOf(c.folded, from)
      if (start < 0) break
      from = start + 1
      const end = start + c.folded.length
      if (isLetter(folded[start - 1]) || isLetter(folded[end])) continue
      if (c.caseSensitive && text.slice(start, end) !== c.raw) continue
      if (taken.some((m) => start < m.end && end > m.start)) continue
      taken.push({ term: c.term, text: text.slice(start, end), start, end })
      break
    }
  }
  return taken.sort((a, b) => a.start - b.start)
}
