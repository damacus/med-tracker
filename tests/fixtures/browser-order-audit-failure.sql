ALTER TABLE public.versions ADD CONSTRAINT synthetic_order_audit_failure
CHECK (event NOT IN ('mark_as_ordered','mark_as_received')) NOT VALID;
