CREATE TABLE public.pg_loco_queue (
    id VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    task_data JSONB NOT NULL,
    status VARCHAR NOT NULL DEFAULT 'queued',
    run_at TIMESTAMPTZ NOT NULL,
    interval BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    tags JSONB,
    priority INT NOT NULL DEFAULT 0
);
REVOKE ALL ON public.pg_loco_queue FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE, DELETE ON public.pg_loco_queue TO med_tracker_app;
