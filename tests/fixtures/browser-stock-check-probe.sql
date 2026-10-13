SELECT json_build_object(
  'scalar_supply', (SELECT current_supply::text FROM public.medications WHERE id=80001),
  'option_supply', (SELECT current_supply::text FROM public.dosages WHERE id=82021),
  'option_parent_supply', (SELECT current_supply::text FROM public.medications WHERE id=80021),
  'untracked_parent_supply', (SELECT current_supply::text FROM public.medications WHERE id=80031),
  'scalar_reason_audits', (SELECT count(*) FROM public.versions WHERE item_type='Medication' AND item_id=80001 AND event LIKE '%reason: house_stock_check%'),
  'option_reason_audits', (SELECT count(*) FROM public.versions WHERE item_type='MedicationDosageOption' AND item_id=82021 AND event LIKE '%reason: house_stock_check%')
);
