import type { Analysis } from '../types/Analysis'
import type { Finding } from '../types/Finding'

export const finding: Finding = {
  id: 'TEL-01:i3',
  rule_id: 'TEL-01',
  severity: 'suspeito',
  title: 'Serviço extra que você pode não ter contratado',
  explanation: 'Isso parece um serviço extra que você pode não ter contratado.',
  item_refs: ['i3'],
  original_text: 'CLUBE VZ MUS 12,90',
  page: 1,
  amount_cents: 1290,
  legal_basis: [{ label: 'CDC art. 39, III', url: 'https://www.planalto.gov.br/ccivil_03/leis/l8078compilado.htm' }],
}

export const analysis: Analysis = {
  id: 'a1',
  created_at: '2026-11-01T10:00:00Z',
  expires_at: '2026-12-01T10:00:00Z',
  source: 'demo',
  bill: {
    kind: 'telecom',
    provider_name: 'Vivaz Telecom',
    reference_month: '2026-10',
    due_date: '2026-11-10',
    issue_date: null,
    total_cents: 12980,
    plan_name: 'Vivaz 20GB',
    items: [
      { id: 'i1', description: 'Plano Vivaz 20GB', amount_cents: 11690, category: 'plano', period: null, original_text: 'PLANO VZ 20GB 116,90', page: 1, plain_explanation: 'Seu plano mensal.' },
      { id: 'i3', description: 'Clube Vivaz Música', amount_cents: 1290, category: 'servico_adicional', period: null, original_text: 'CLUBE VZ MUS 12,90', page: 1, plain_explanation: 'Assinatura de música.' },
    ],
    late_fees: null,
    energy: null,
    extraction_confidence: 0.95,
  },
  answers: {},
  paid: null,
  findings: [finding],
  questions: [{ id: 'TEL-01:i3', rule_id: 'TEL-01', item_id: 'i3', text: 'Você contratou Clube Vivaz Música?', kind: 'yes_no' }],
  recoverable_cents: 0,
  rules_version: 'test',
}

const hist = [210, 195, 188, 176, 172, 180, 190, 205, 214, 226, 218, 226]
export const energyAnalysis: Analysis = {
  ...analysis,
  id: 'e1',
  bill: {
    kind: 'energy',
    provider_name: 'Luzia Energia',
    reference_month: '2026-10',
    due_date: '2026-11-10',
    issue_date: '2026-10-30',
    total_cents: 14904 + 11016 + 4666,
    plan_name: null,
    items: [
      { id: 'i1', description: 'Uso da rede (TUSD) 324 kWh', amount_cents: 14904, category: 'energia', period: null, original_text: 'TUSD 324 KWH x 0,46 149,04', page: 1, plain_explanation: 'Custo de levar a energia até sua casa.' },
      { id: 'i2', description: 'Energia consumida (TE) 324 kWh', amount_cents: 11016, category: 'energia', period: null, original_text: 'TE 324 KWH x 0,34 110,16', page: 1, plain_explanation: 'Custo da energia que você usou no mês.' },
      { id: 'i3', description: 'ICMS 18%', amount_cents: 4666, category: 'imposto', period: null, original_text: 'ICMS 18% 46,66', page: 1, plain_explanation: 'Imposto estadual cobrado sobre a energia.' },
    ],
    late_fees: null,
    energy: {
      consumption_kwh: 324,
      reading_type: 'estimada',
      reading_prev_date: '2026-09-28',
      reading_curr_date: '2026-10-28',
      tariff_flag: 'verde',
      history: hist.map((kwh, i) => ({ month: i < 3 ? `2025-${10 + i}` : `2026-${String(i - 2).padStart(2, '0')}`, kwh })),
    },
    extraction_confidence: 0.95,
  },
  findings: [],
  questions: [],
}
