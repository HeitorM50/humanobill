import type { Analysis } from '../types/Analysis'
import type { Finding } from '../types/Finding'

export const finding: Finding = {
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
