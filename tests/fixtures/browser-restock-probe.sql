SELECT json_build_object(
  'supply', (SELECT current_supply::text FROM public.medications WHERE id=80001),
  'last_restock', (SELECT supply_at_last_restock::text FROM public.medications WHERE id=80001),
  'order_status', (SELECT reorder_status FROM public.medications WHERE id=80001),
  'restock_audits', (SELECT count(*) FROM public.versions WHERE item_type='Medication' AND item_id=80001 AND event LIKE 'restock (qty:%')
);
