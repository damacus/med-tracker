ALTER TABLE public.stock_allocations ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.stock_allocations FORCE ROW LEVEL SECURITY;
CREATE POLICY household_tenant_isolation ON public.stock_allocations
    USING (household_id = med_tracker.current_household_id())
    WITH CHECK (household_id = med_tracker.current_household_id());
GRANT SELECT, INSERT, UPDATE (remaining_quantity) ON public.stock_allocations TO med_tracker_app;
GRANT USAGE, SELECT ON SEQUENCE public.stock_allocations_id_seq TO med_tracker_app;

CREATE FUNCTION public.protect_allocated_medication_stock() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, pg_temp
AS $$
DECLARE
    reserved numeric;
BEGIN
    SELECT coalesce(sum(allocation.remaining_quantity), 0) INTO reserved
    FROM public.stock_allocations AS allocation
    WHERE allocation.household_id = OLD.household_id
      AND allocation.medication_id = OLD.id
      AND allocation.remaining_quantity > 0;
    IF TG_OP = 'DELETE' THEN
        IF reserved > 0 THEN
            RAISE EXCEPTION 'Stock remains allocated to a person' USING ERRCODE = '23514';
        END IF;
        RETURN OLD;
    END IF;
    IF reserved > 0 AND (
        coalesce(NEW.current_supply, 0) < reserved
        OR ROW(NEW.dmd_system, NEW.dmd_concept_class, NEW.dmd_code, NEW.dose_unit)
           IS DISTINCT FROM ROW(OLD.dmd_system, OLD.dmd_concept_class, OLD.dmd_code, OLD.dose_unit)
    ) THEN
        RAISE EXCEPTION 'Stock remains allocated to a person' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER protect_allocated_medication_stock
    BEFORE UPDATE OF current_supply, dmd_system, dmd_concept_class, dmd_code, dose_unit OR DELETE
    ON public.medications FOR EACH ROW EXECUTE FUNCTION public.protect_allocated_medication_stock();

CREATE FUNCTION public.protect_allocated_dosage_stock() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, pg_temp
AS $$
DECLARE
    reserved numeric;
BEGIN
    IF TG_OP <> 'INSERT' THEN
        SELECT coalesce(sum(allocation.remaining_quantity), 0) INTO reserved
        FROM public.stock_allocations AS allocation
        WHERE allocation.household_id = OLD.household_id
          AND allocation.medication_id = OLD.medication_id
          AND allocation.dosage_id = OLD.id
          AND allocation.remaining_quantity > 0;
        IF TG_OP = 'DELETE' THEN
            IF reserved > 0 THEN
                RAISE EXCEPTION 'Stock remains allocated to a person' USING ERRCODE = '23514';
            END IF;
            RETURN OLD;
        END IF;
        IF reserved > 0 AND (
            coalesce(NEW.current_supply, 0) < reserved
            OR ROW(NEW.medication_id, NEW.household_id, NEW.amount, NEW.unit)
               IS DISTINCT FROM ROW(OLD.medication_id, OLD.household_id, OLD.amount, OLD.unit)
        ) THEN
            RAISE EXCEPTION 'Stock remains allocated to a person' USING ERRCODE = '23514';
        END IF;
    END IF;
    IF NEW.current_supply IS NOT NULL AND EXISTS (
        SELECT 1 FROM public.stock_allocations AS allocation
        WHERE allocation.household_id = NEW.household_id
          AND allocation.medication_id = NEW.medication_id
          AND allocation.dosage_id IS NULL
          AND allocation.remaining_quantity > 0
    ) THEN
        RAISE EXCEPTION 'Medication stock remains allocated to a person' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER protect_allocated_dosage_stock
    BEFORE INSERT OR UPDATE OF current_supply, medication_id, household_id, amount, unit OR DELETE
    ON public.dosages FOR EACH ROW EXECUTE FUNCTION public.protect_allocated_dosage_stock();
