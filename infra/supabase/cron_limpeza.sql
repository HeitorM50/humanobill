-- RF14: apaga diariamente as análises expiradas (30 dias).
-- Específico do Supabase (pg_cron); não roda no Postgres local nem pelo sqlx.
-- Aplicado no projeto "humanobill" em 11/11/2026. Para reaplicar: SQL Editor do Supabase.
create extension if not exists pg_cron;

select cron.unschedule(jobid) from cron.job where jobname = 'limpar-analises-expiradas';

select cron.schedule(
  'limpar-analises-expiradas',
  '15 3 * * *',
  $$delete from public.analyses where expires_at < now()$$
);
