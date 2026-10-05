WITH namespaces AS (
    SELECT * FROM pg_namespace
    WHERE nspname !~ '^pg_' AND nspname <> 'information_schema'
), objects AS (
    SELECT 'schema:' || nspname AS identity,
        jsonb_build_object('owner', pg_get_userbyid(nspowner), 'acl',
            (SELECT jsonb_agg(x::text ORDER BY x::text) FROM unnest(nspacl) x)) AS definition
    FROM namespaces
    UNION ALL
    SELECT 'extension:' || e.extname,
        jsonb_build_object('schema', n.nspname, 'version', e.extversion,
            'owner', pg_get_userbyid(e.extowner), 'relocatable', e.extrelocatable)
    FROM pg_extension e JOIN pg_namespace n ON n.oid = e.extnamespace
    UNION ALL
    SELECT 'relation:' || n.nspname || '.' || c.relname,
        jsonb_build_object('kind', c.relkind, 'owner', pg_get_userbyid(c.relowner),
            'persistence', c.relpersistence, 'rls', c.relrowsecurity, 'force_rls', c.relforcerowsecurity,
            'replica_identity', c.relreplident, 'partition', c.relispartition,
            'partition_bound', pg_get_expr(c.relpartbound, c.oid),
            'partition_key', CASE WHEN c.relkind = 'p' THEN pg_get_partkeydef(c.oid) END,
            'access_method', am.amname, 'tablespace', ts.spcname,
            'options', (SELECT jsonb_agg(x ORDER BY x) FROM unnest(c.reloptions) x),
            'acl', (SELECT jsonb_agg(x::text ORDER BY x::text) FROM unnest(c.relacl) x))
    FROM pg_class c JOIN namespaces n ON n.oid = c.relnamespace
    LEFT JOIN pg_am am ON am.oid = c.relam LEFT JOIN pg_tablespace ts ON ts.oid = c.reltablespace
    WHERE c.relkind <> 'i' AND c.relkind <> 'I'
    UNION ALL
    SELECT 'column:' || n.nspname || '.' || c.relname || '.' || a.attname,
        jsonb_build_object('position', a.attnum, 'type', format_type(a.atttypid, a.atttypmod),
            'not_null', a.attnotnull, 'identity', a.attidentity, 'generated', a.attgenerated,
            'default', pg_get_expr(d.adbin, d.adrelid),
            'collation', CASE WHEN a.attcollation <> 0 THEN cn.nspname || '.' || co.collname END,
            'storage', a.attstorage, 'compression', a.attcompression,
            'acl', (SELECT jsonb_agg(x::text ORDER BY x::text) FROM unnest(a.attacl) x),
            'options', (SELECT jsonb_agg(x ORDER BY x) FROM unnest(a.attoptions) x))
    FROM pg_attribute a JOIN pg_class c ON c.oid = a.attrelid JOIN namespaces n ON n.oid = c.relnamespace
    LEFT JOIN pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum
    LEFT JOIN pg_collation co ON co.oid = a.attcollation LEFT JOIN pg_namespace cn ON cn.oid = co.collnamespace
    WHERE a.attnum > 0 AND NOT a.attisdropped AND c.relkind NOT IN ('i', 'I')
    UNION ALL
    SELECT 'constraint:' || n.nspname || '.' || COALESCE(c.relname, t.typname) || '.' || k.conname,
        jsonb_build_object('definition', pg_get_constraintdef(k.oid, false), 'validated', k.convalidated,
            'enforced', k.conenforced,
            'deferrable', k.condeferrable, 'deferred', k.condeferred, 'local', k.conislocal,
            'inherit_count', k.coninhcount, 'no_inherit', k.connoinherit)
    FROM pg_constraint k JOIN namespaces n ON n.oid = k.connamespace
    LEFT JOIN pg_class c ON c.oid = k.conrelid LEFT JOIN pg_type t ON t.oid = k.contypid
    UNION ALL
    SELECT 'index:' || n.nspname || '.' || c.relname,
        jsonb_build_object('definition', pg_get_indexdef(c.oid), 'owner', pg_get_userbyid(c.relowner),
            'valid', i.indisvalid, 'ready', i.indisready, 'live', i.indislive,
            'replica_identity', i.indisreplident, 'clustered', i.indisclustered,
            'tablespace', ts.spcname,
            'options', (SELECT jsonb_agg(x ORDER BY x) FROM unnest(c.reloptions) x))
    FROM pg_index i JOIN pg_class c ON c.oid = i.indexrelid JOIN namespaces n ON n.oid = c.relnamespace
    LEFT JOIN pg_tablespace ts ON ts.oid = c.reltablespace
    UNION ALL
    SELECT 'function:' || n.nspname || '.' || p.proname || '(' || pg_get_function_identity_arguments(p.oid) || ')',
        jsonb_build_object('definition', CASE WHEN p.prokind <> 'a' THEN pg_get_functiondef(p.oid) END,
            'owner', pg_get_userbyid(p.proowner),
            'acl', (SELECT jsonb_agg(x::text ORDER BY x::text) FROM unnest(p.proacl) x))
    FROM pg_proc p JOIN namespaces n ON n.oid = p.pronamespace
    UNION ALL
    SELECT 'trigger:' || n.nspname || '.' || c.relname || '.' || t.tgname,
        jsonb_build_object('definition', pg_get_triggerdef(t.oid, false), 'enabled', t.tgenabled)
    FROM pg_trigger t JOIN pg_class c ON c.oid = t.tgrelid JOIN namespaces n ON n.oid = c.relnamespace
    WHERE NOT t.tgisinternal
    UNION ALL
    SELECT 'internal_trigger:' || n.nspname || '.' || c.relname || ':' ||
        kn.nspname || '.' || kc.relname || '.' || k.conname || ':' || t.tgtype::text,
        jsonb_build_object('function', t.tgfoid::regprocedure::text,
            'enabled', t.tgenabled, 'type', t.tgtype,
            'deferrable', t.tgdeferrable, 'deferred', t.tginitdeferred)
    FROM pg_trigger t JOIN pg_class c ON c.oid = t.tgrelid JOIN namespaces n ON n.oid = c.relnamespace
    JOIN pg_constraint k ON k.oid = t.tgconstraint
    JOIN pg_class kc ON kc.oid = k.conrelid JOIN pg_namespace kn ON kn.oid = kc.relnamespace
    WHERE t.tgisinternal
    UNION ALL
    SELECT 'rule:' || n.nspname || '.' || c.relname || '.' || r.rulename,
        jsonb_build_object('definition', pg_get_ruledef(r.oid, false), 'enabled', r.ev_enabled)
    FROM pg_rewrite r JOIN pg_class c ON c.oid = r.ev_class JOIN namespaces n ON n.oid = c.relnamespace
    UNION ALL
    SELECT 'policy:' || n.nspname || '.' || c.relname || '.' || p.polname,
        jsonb_build_object('command', p.polcmd, 'permissive', p.polpermissive,
            'roles', (SELECT jsonb_agg(CASE WHEN r = 0 THEN 'PUBLIC' ELSE pg_get_userbyid(r)::text END ORDER BY r::regrole::text) FROM unnest(p.polroles) r),
            'using', pg_get_expr(p.polqual, p.polrelid), 'check', pg_get_expr(p.polwithcheck, p.polrelid))
    FROM pg_policy p JOIN pg_class c ON c.oid = p.polrelid JOIN namespaces n ON n.oid = c.relnamespace
    UNION ALL
    SELECT 'sequence:' || n.nspname || '.' || c.relname,
        jsonb_build_object('type', format_type(s.seqtypid, NULL), 'start', s.seqstart,
            'increment', s.seqincrement, 'minimum', s.seqmin, 'maximum', s.seqmax,
            'cache', s.seqcache, 'cycle', s.seqcycle,
            'owned_by', (SELECT rn.nspname || '.' || rc.relname || '.' || a.attname
                FROM pg_depend d JOIN pg_class rc ON rc.oid = d.refobjid
                JOIN pg_namespace rn ON rn.oid = rc.relnamespace
                JOIN pg_attribute a ON a.attrelid = rc.oid AND a.attnum = d.refobjsubid
                WHERE d.classid = 'pg_class'::regclass AND d.objid = c.oid
                  AND d.refclassid = 'pg_class'::regclass AND d.deptype IN ('a', 'i')))
    FROM pg_sequence s JOIN pg_class c ON c.oid = s.seqrelid JOIN namespaces n ON n.oid = c.relnamespace
    UNION ALL
    SELECT 'default_acl:' || pg_get_userbyid(d.defaclrole) || ':' || COALESCE(n.nspname, '*') || ':' || d.defaclobjtype::text,
        jsonb_build_object('acl', (SELECT jsonb_agg(x::text ORDER BY x::text) FROM unnest(d.defaclacl) x))
    FROM pg_default_acl d LEFT JOIN pg_namespace n ON n.oid = d.defaclnamespace
    UNION ALL
    SELECT 'type:' || n.nspname || '.' || t.typname,
        jsonb_build_object('kind', t.typtype, 'owner', pg_get_userbyid(t.typowner),
            'base', format_type(t.typbasetype, t.typtypmod), 'not_null', t.typnotnull,
            'default', t.typdefault, 'category', t.typcategory,
            'enum', (SELECT jsonb_agg(e.enumlabel ORDER BY e.enumsortorder) FROM pg_enum e WHERE e.enumtypid = t.oid),
            'acl', (SELECT jsonb_agg(x::text ORDER BY x::text) FROM unnest(t.typacl) x))
    FROM pg_type t JOIN namespaces n ON n.oid = t.typnamespace
    WHERE t.typrelid = 0 AND t.typelem = 0
    UNION ALL
    SELECT 'inheritance:' || cn.nspname || '.' || c.relname || ':' || pn.nspname || '.' || p.relname,
        jsonb_build_object('sequence', i.inhseqno, 'detach_pending', i.inhdetachpending)
    FROM pg_inherits i JOIN pg_class c ON c.oid = i.inhrelid JOIN namespaces cn ON cn.oid = c.relnamespace
    JOIN pg_class p ON p.oid = i.inhparent JOIN pg_namespace pn ON pn.oid = p.relnamespace
    UNION ALL
    SELECT 'event_trigger:' || evtname,
        jsonb_build_object('event', evtevent, 'owner', pg_get_userbyid(evtowner),
            'function', evtfoid::regprocedure::text, 'enabled', evtenabled, 'tags', evttags)
    FROM pg_event_trigger
)
SELECT jsonb_object_agg(identity, definition ORDER BY identity)::text AS catalog FROM objects
