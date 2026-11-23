import type { Analysis } from '../../types/Analysis'

/**
 * De onde vem a imagem da fatura para a marcação visual (#51).
 *
 * - Exemplo (demo): o PNG público em /exemplos, achado pela "impressão digital"
 *   da fatura (empresa, mês e total). As caixas das fixtures foram medidas
 *   nesse PNG.
 * - Upload de imagem: o `File` que o navegador ainda tem, guardado só na
 *   memória desta aba (object URL). Nada vai para o servidor nem para o disco.
 * - Upload de PDF: sem imagem (desenhar o PDF exigiria o pdf.js).
 */

/** Exemplos com PNG em public/exemplos. Chave: empresa|mês|total em centavos. */
const SAMPLE_PNGS: Record<string, string> = {
  'Vivaz Telecom|2026-10|12270': 'telecom-1',
  'Vivaz Telecom|2026-09|9990': 'telecom-1-anterior',
  'Vivaz Fibra|2026-10|13950': 'telecom-2',
  'Vivaz Telecom|2026-10|13990': 'telecom-3',
  'Luzia Energia|2026-10|33182': 'energia-1',
  'Luzia Energia|2026-10|23158': 'energia-2',
  'Luzia Energia|2026-10|21631': 'energia-3',
  'Banco Aurora|2026-10|6130': 'extrato-1',
  'Aurora Card|2026-10|126803': 'cartao-1',
}

// Só a última imagem enviada fica guardada: a anterior é liberada.
const uploaded = new Map<string, string>()

/** Guarda a imagem enviada para mostrar no resultado. PDF é ignorado. */
export function rememberUploadedBill(analysisId: string, file: File): void {
  if (!file.type.startsWith('image/')) return
  for (const url of uploaded.values()) URL.revokeObjectURL(url)
  uploaded.clear()
  uploaded.set(analysisId, URL.createObjectURL(file))
}

/** Apaga a imagem guardada (testes). */
export function forgetUploadedBills(): void {
  for (const url of uploaded.values()) URL.revokeObjectURL(url)
  uploaded.clear()
}

/** URL da imagem da página 1 da fatura, ou `null` se não há imagem. */
export function billImageSrc(analysis: Analysis): string | null {
  const local = uploaded.get(analysis.id)
  if (local) return local
  if (analysis.source !== 'demo') return null
  const { provider_name, reference_month, total_cents } = analysis.bill
  const sample = SAMPLE_PNGS[`${provider_name}|${reference_month}|${total_cents}`]
  return sample ? `/exemplos/${sample}.png` : null
}
