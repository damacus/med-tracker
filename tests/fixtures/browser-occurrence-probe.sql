SELECT json_build_object(
  'occurrences', (SELECT count(*) FROM public.medication_dose_occurrences WHERE household_id=72001),
  'not_taken', (SELECT count(*) FROM public.medication_dose_occurrences WHERE household_id=72001 AND outcome='not_taken'),
  'taken', (SELECT count(*) FROM public.medication_dose_occurrences WHERE household_id=72001 AND outcome='taken'),
  'open', (SELECT count(*) FROM public.medication_dose_occurrences WHERE household_id=72001 AND outcome='open'),
  'occurrence_versions', (SELECT count(*) FROM public.versions WHERE household_id=72001 AND item_type='MedicationDoseOccurrence'),
  'occurrence_changes', (SELECT count(*) FROM public.api_change_events WHERE household_id=72001 AND record_type='MedicationDoseOccurrence'),
  'take_versions', (SELECT count(*) FROM public.versions WHERE household_id=72001 AND item_type='MedicationTake'),
  'takes', (SELECT count(*) FROM public.medication_takes WHERE household_id=72001),
  'supply', (SELECT current_supply::text FROM public.medications WHERE id=80001)
);
