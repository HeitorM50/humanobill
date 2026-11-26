import { describe, expect, it } from 'vitest'
import { findTermsIn, GLOSSARY, searchGlossary } from './glossary'

const ids = (text: string) => findTermsIn(text).map((m) => m.term.id)

describe('RF24: lista do glossário', () => {
  it('traz os termos mínimos, com id único e texto preenchido', () => {
    const esperados = ['te', 'tusd', 'cip', 'icms', 'pis-cofins', 'sva', 'bandeira', 'leitura-estimada', 'kwh', 'multa-juros', 'tarifa-essenciais', 'anuidade']
    for (const id of esperados) expect(GLOSSARY.map((t) => t.id)).toContain(id)
    expect(new Set(GLOSSARY.map((t) => t.id)).size).toBe(GLOSSARY.length)
    for (const t of GLOSSARY) {
      expect(t.term).not.toBe('')
      expect(t.short).not.toBe('')
      expect(t.long).not.toBe('')
    }
  })

  it('não cita valor em reais por kWh (eles mudam)', () => {
    for (const t of GLOSSARY) expect(`${t.short} ${t.long}`).not.toMatch(/R\$/)
  })

  it('SVA diz que precisa de autorização prévia e expressa (RGC art. 48)', () => {
    const sva = GLOSSARY.find((t) => t.id === 'sva')!
    expect(sva.long).toMatch(/autorização prévia e expressa/)
    expect(sva.long).toMatch(/art\. 48/)
  })
})

describe('findTermsIn', () => {
  it('acha TUSD e kWh em "Uso da rede (TUSD) 198 kWh"', () => {
    expect(ids('Uso da rede (TUSD) 198 kWh')).toEqual(['tusd', 'kwh'])
  })

  it('acha TE e kWh em "Energia consumida (TE) 205 kWh"', () => {
    expect(ids('Energia consumida (TE) 205 kWh')).toEqual(['te', 'kwh'])
  })

  it('acha a bandeira em "Bandeira tarifária vermelha patamar 1"', () => {
    expect(ids('Bandeira tarifária vermelha patamar 1')).toEqual(['bandeira'])
  })

  it('acha a CIP em "Contribuição de iluminação pública"', () => {
    expect(ids('Contribuição de iluminação pública')).toEqual(['cip'])
  })

  it('acha multa e juros em "Multa por atraso" e "Juros por atraso" (fatura energia-2)', () => {
    expect(ids('Multa por atraso')).toEqual(['multa-juros'])
    expect(ids('Juros por atraso')).toEqual(['multa-juros'])
  })

  it('acha PIS/COFINS e ICMS', () => {
    expect(ids('PIS/COFINS')).toEqual(['pis-cofins'])
    expect(ids('ICMS 18%')).toEqual(['icms'])
  })

  it('devolve a posição e o trecho original, com acento e caixa como estavam', () => {
    const [m] = findTermsIn('Contribuição de iluminação pública')
    expect(m.text).toBe('Contribuição de iluminação pública')
    expect(m.start).toBe(0)
    expect(m.end).toBe(34)
  })

  it('casa sem depender de acento nem de caixa', () => {
    expect(ids('CONTRIBUICAO DE ILUMINACAO PUBLICA')).toEqual(['cip'])
    expect(ids('leitura ESTIMADA')).toEqual(['leitura-estimada'])
  })

  it('usa limite de palavra: TE não casa dentro de TELEFONE nem de "Vivaz Telecom"', () => {
    expect(ids('TELEFONE FIXO')).toEqual([])
    expect(ids('Vivaz Telecom')).toEqual([])
    expect(ids('Serviço de ATENDIMENTO')).toEqual([])
  })

  it('não casa nada em "Plano Vivaz Controle 20GB"', () => {
    expect(ids('Plano Vivaz Controle 20GB')).toEqual([])
  })

  it('o "te" minúsculo de uma frase não vira o termo TE', () => {
    expect(ids('Aviso para te lembrar')).toEqual([])
  })

  it('aceita kWh colado no número', () => {
    expect(ids('Consumo 198kWh')).toEqual(['kwh'])
  })

  it('prefere o trecho mais longo e não repete o mesmo termo', () => {
    const m = findTermsIn('Bandeira tarifária amarela e bandeira amarela')
    expect(m).toHaveLength(1)
    expect(m[0].text).toBe('Bandeira tarifária')
  })

  it('devolve em ordem de aparição', () => {
    expect(ids('ICMS e PIS/COFINS e TUSD')).toEqual(['icms', 'pis-cofins', 'tusd'])
  })
})

describe('searchGlossary', () => {
  it('sem busca devolve tudo', () => {
    expect(searchGlossary('')).toHaveLength(GLOSSARY.length)
    expect(searchGlossary('   ')).toHaveLength(GLOSSARY.length)
  })

  it('"bandeira" traz a explicação das bandeiras verde, amarela e vermelha', () => {
    const r = searchGlossary('bandeira')
    expect(r.map((t) => t.id)).toContain('bandeira')
    const b = r.find((t) => t.id === 'bandeira')!
    expect(b.long).toMatch(/verde/i)
    expect(b.long).toMatch(/amarela/i)
    expect(b.long).toMatch(/vermelha/i)
    expect(b.long).toMatch(/patamar 1/i)
    expect(b.long).toMatch(/patamar 2/i)
    expect(b.long).toMatch(/ANEEL/)
  })

  it('ignora acento e caixa', () => {
    expect(searchGlossary('ILUMINACAO').map((t) => t.id)).toContain('cip')
    expect(searchGlossary('serviço de valor').map((t) => t.id)).toContain('sva')
  })

  it('procura também nos apelidos e no texto', () => {
    expect(searchGlossary('cosip').map((t) => t.id)).toContain('cip')
    expect(searchGlossary('prefeitura').map((t) => t.id)).toContain('cip')
  })

  it('devolve vazio quando nada casa', () => {
    expect(searchGlossary('xyzzy')).toEqual([])
  })
})
