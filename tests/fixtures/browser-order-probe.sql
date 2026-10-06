SELECT json_build_object(
  'status', (SELECT reorder_status FROM public.medications WHERE id=80001),
  'supply', (SELECT current_supply::text FROM public.medications WHERE id=80001),
  'supplier', (SELECT order_supplier FROM public.medications WHERE id=80001),
  'quantity', (SELECT order_quantity::text FROM public.medications WHERE id=80001),
  'arrival', (SELECT expected_arrival_on::text FROM public.medications WHERE id=80001),
  'ordered_audits', (SELECT count(*) FROM public.versions WHERE item_type='Medication' AND item_id=80001 AND event='mark_as_ordered'),
  'received_audits', (SELECT count(*) FROM public.versions WHERE item_type='Medication' AND item_id=80001 AND event='mark_as_received')
);
