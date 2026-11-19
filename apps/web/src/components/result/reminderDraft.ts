import type { ReminderCreated } from '../../types/ReminderCreated'

/** O que a pessoa já digitou (e se já agendou). Fica só na memória: o e-mail nunca vai para o navegador. */
export type ReminderDraft = { email: string; consent: boolean; created: ReminderCreated | null }
export const EMPTY_REMINDER_DRAFT: ReminderDraft = { email: '', consent: false, created: null }
