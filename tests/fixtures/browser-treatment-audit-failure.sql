ALTER TABLE public.versions ADD CONSTRAINT synthetic_treatment_audit_failure
CHECK (item_type NOT IN ('Schedule','PersonMedication','MedicationPausePeriod')) NOT VALID;
