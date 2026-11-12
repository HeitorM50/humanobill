import { useSyncExternalStore } from 'react'
import type { Analysis } from '../types/Analysis'
import type { Answer } from '../types/Answer'
import type { Dispute } from '../types/Dispute'
import type { ExtractedBill } from '../types/ExtractedBill'

export const API_URL: string = import.meta.env.VITE_API_URL ?? 'http://localhost:8099'

/** Exemplos disponíveis. `file` é o PDF em public/exemplos usado na extração ao vivo. */
export const DEMO_SAMPLES = [
  { id: 'telecom-1', title: 'Celular com serviços extras', kind: 'Telecom', group: 'Celular/Internet', file: 'telecom-1.pdf' },
  { id: 'telecom-2', title: 'Internet com multa e juros', kind: 'Telecom', group: 'Celular/Internet', file: 'telecom-2.pdf' },
  { id: 'telecom-3', title: 'Conta sem problemas', kind: 'Telecom', group: 'Celular/Internet', file: 'telecom-3.pdf' },
  { id: 'energia-1', title: 'Conta de luz alta e leitura estimada', kind: 'Energia', group: 'Luz', file: 'energia-1.pdf' },
  { id: 'energia-2', title: 'Seguro embutido e multa', kind: 'Energia', group: 'Luz', file: 'energia-2.pdf' },
  { id: 'energia-3', title: 'Bandeira cobrada errada', kind: 'Energia', group: 'Luz', file: 'energia-3.pdf' },
] as const

export const DEMO_GROUPS = ['Celular/Internet', 'Luz'] as const

export class ApiError extends Error {
  status: number
  code: string
  constructor(status: number, code: string, message: string) {
    super(message)
    this.status = status
    this.code = code
  }
}

// Contador de requisições do usuário em andamento (para o aviso de cold start).
let pending = 0
let slow = false
let timer: ReturnType<typeof setTimeout> | undefined
const listeners = new Set<() => void>()
const emit = () => listeners.forEach((l) => l())

function begin() {
  pending++
  if (pending === 1) {
    timer = setTimeout(() => {
      slow = true
      emit()
    }, 2000)
  }
}
function end() {
  pending = Math.max(0, pending - 1)
  if (pending === 0) {
    clearTimeout(timer)
    if (slow) {
      slow = false
      emit()
    }
  }
}

/** true quando alguma requisição passa de 2 s (servidor acordando). */
export function useServerWaking(): boolean {
  return useSyncExternalStore(
    (cb) => {
      listeners.add(cb)
      return () => listeners.delete(cb)
    },
    () => slow,
  )
}

async function request<T>(path: string, init?: RequestInit, track = true): Promise<T> {
  if (track) begin()
  try {
    let res: Response
    try {
      res = await fetch(`${API_URL}${path}`, init)
    } catch {
      throw new ApiError(0, 'network', 'Não consegui falar com o servidor. Tente de novo em instantes.')
    }
    if (!res.ok) {
      let code = 'unknown'
      let message = 'Algo deu errado. Tente de novo.'
      try {
        const body = await res.json()
        code = body?.error?.code ?? code
        message = body?.error?.message ?? message
      } catch {
        /* corpo sem JSON */
      }
      throw new ApiError(res.status, code, message)
    }
    return (await res.json()) as T
  } finally {
    if (track) end()
  }
}

export const warmUp = () => request<{ status: string }>('/health', undefined, false).catch(() => undefined)

export const createDemoAnalysis = (sample: string) =>
  request<Analysis>(`/analyses/demo/${encodeURIComponent(sample)}`, { method: 'POST' })

export function uploadAnalysis(file: File): Promise<Analysis> {
  const form = new FormData()
  form.append('consent', 'true')
  form.append('file', file)
  return request<Analysis>('/analyses', { method: 'POST', body: form })
}

/** Baixa o PDF do exemplo e envia pelo upload real (extração com IA). */
export async function uploadSample(sampleId: string): Promise<Analysis> {
  const sample = DEMO_SAMPLES.find((s) => s.id === sampleId)
  if (!sample) throw new ApiError(0, 'unknown_sample', 'Não achei esse exemplo.')
  let blob: Blob
  try {
    const res = await fetch(`/exemplos/${sample.file}`)
    if (!res.ok) throw new Error('http')
    blob = await res.blob()
  } catch {
    throw new ApiError(0, 'sample_unavailable', 'Não consegui abrir o arquivo de exemplo.')
  }
  return uploadAnalysis(new File([blob], sample.file, { type: 'application/pdf' }))
}

export const getAnalysis = (id: string) => request<Analysis>(`/analyses/${encodeURIComponent(id)}`)

/** Corpo do PATCH. `answers` SUBSTITUI o mapa inteiro: envie todas as respostas atuais. */
export type AnalysisPatch = { answers?: Record<string, Answer>; paid?: boolean | null; bill?: ExtractedBill }

export const patchAnalysis = (id: string, body: AnalysisPatch) =>
  request<Analysis>(`/analyses/${encodeURIComponent(id)}`, {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  })

export type DisputeRequest = { finding_ids: string[]; protocol?: string }

export const createDispute = (id: string, body: DisputeRequest) =>
  request<Dispute>(`/analyses/${encodeURIComponent(id)}/dispute`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  })
