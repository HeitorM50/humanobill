/** Caminho da carta formal (impressão) de uma contestação: os achados e o protocolo vão na query string. */
export function letterPath(analysisId: string, findingIds: string[], protocol = ''): string {
  const q = new URLSearchParams()
  for (const id of findingIds) q.append('item', id)
  if (protocol.trim()) q.set('protocolo', protocol.trim())
  return `/r/${encodeURIComponent(analysisId)}/carta?${q.toString()}`
}

/** Na carta, o "Olá." do texto do chat vira a saudação formal "Prezados,". O resto do texto não muda. */
export function formalLetterText(text: string): string {
  return text.replace(/^\s*Olá[.,!]?\s*/, 'Prezados,\n\n')
}
