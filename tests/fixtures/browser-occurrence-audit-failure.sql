ALTER TABLE public.versions ADD CONSTRAINT browser_occurrence_audit_failure CHECK (item_type <> 'MedicationDoseOccurrence') NOT VALID;
