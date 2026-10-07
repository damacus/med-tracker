INSERT INTO public.health_events(household_id,person_id,event_kind,title,started_on,ended_on,severity,notes,action_taken,medical_help_sought,created_at,updated_at)
SELECT 72001,73001,number % 2,'Representative symptom ' || number,date '2026-01-01' + ((number - 1) % 31),date '2026-01-01' + ((number - 1) % 31),number % 3,'Recorded context for event ' || number,'Discussed with care team',false,now(),now()
FROM generate_series(1,80) AS event(number);
