# frozen_string_literal: true

class RepairRlsHiddenAuditSources < ActiveRecord::Migration[8.1]
  def up
    execute <<~SQL
      LOCK TABLE versions, security_audit_events IN SHARE MODE;

      CREATE POLICY repair_rls_hidden_audit_sources_select ON versions
        FOR SELECT TO med_tracker_owner USING (true);
      CREATE POLICY repair_rls_hidden_audit_sources_select ON security_audit_events
        FOR SELECT TO med_tracker_owner USING (true);

      INSERT INTO audit_chain_heads (chain_key, household_id, epoch_kind, created_at, updated_at)
      SELECT DISTINCT sources.chain_key, sources.household_id, 'live', clock_timestamp(), clock_timestamp()
      FROM (
        SELECT COALESCE('household:' || household_id::text, 'global') chain_key,
               household_id, 'versions'::text source_table, id source_id
        FROM versions
        UNION ALL
        SELECT COALESCE('household:' || household_id::text, 'global'),
               household_id, 'security_audit_events'::text, id
        FROM security_audit_events
      ) sources
      WHERE NOT EXISTS (
        SELECT 1
        FROM audit_ledger_entries entries
        WHERE entries.source_table = sources.source_table AND entries.source_id = sources.source_id
      )
      ON CONFLICT (chain_key) DO NOTHING;

      DO $$
      DECLARE
        chain_row record;
        source_row record;
        chain_head audit_chain_heads%ROWTYPE;
      BEGIN
        FOR chain_row IN
          SELECT sources.chain_key, sources.household_id
          FROM (
            SELECT COALESCE('household:' || household_id::text, 'global') chain_key,
                   household_id, 'versions'::text source_table, id source_id
            FROM versions
            UNION ALL
            SELECT COALESCE('household:' || household_id::text, 'global'),
                   household_id, 'security_audit_events'::text, id
            FROM security_audit_events
          ) sources
          WHERE NOT EXISTS (
            SELECT 1
            FROM audit_ledger_entries entries
            WHERE entries.source_table = sources.source_table AND entries.source_id = sources.source_id
          )
          GROUP BY sources.chain_key, sources.household_id
          ORDER BY sources.chain_key
        LOOP
          SELECT * INTO STRICT chain_head
          FROM audit_chain_heads
          WHERE chain_key = chain_row.chain_key
          FOR UPDATE;

          IF chain_head.epoch_kind = 'live' AND chain_head.last_sequence > 0 THEN
            INSERT INTO audit_checkpoints (
              household_id, chain_key, chain_epoch, checkpoint_kind, sequence, entry_hash,
              created_at, updated_at
            ) VALUES (
              chain_head.household_id, chain_head.chain_key, chain_head.chain_epoch, 'pre-legacy-repair',
              chain_head.last_sequence, chain_head.last_hash, clock_timestamp(), clock_timestamp()
            )
            ON CONFLICT (chain_key, chain_epoch, sequence) DO NOTHING;

            IF NOT EXISTS (
              SELECT 1
              FROM audit_checkpoints checkpoints
              WHERE checkpoints.chain_key = chain_head.chain_key
                AND checkpoints.chain_epoch = chain_head.chain_epoch
                AND checkpoints.sequence = chain_head.last_sequence
                AND checkpoints.household_id IS NOT DISTINCT FROM chain_head.household_id
                AND checkpoints.entry_hash = chain_head.last_hash
            ) THEN
              RAISE EXCEPTION 'existing checkpoint does not match the pre-repair live tail';
            END IF;
          END IF;

          UPDATE audit_chain_heads
          SET chain_epoch = gen_random_uuid(), epoch_kind = 'legacy-repair',
              last_sequence = 0, last_hash = NULL, updated_at = clock_timestamp()
          WHERE id = chain_head.id;

          FOR source_row IN
            SELECT source_table, source_id, household_id, source_payload, occurred_at
            FROM (
              SELECT 'versions'::text source_table, id source_id, household_id,
                     to_jsonb(versions.*) source_payload, created_at occurred_at
              FROM versions
              UNION ALL
              SELECT 'security_audit_events'::text, id, household_id,
                     to_jsonb(security_audit_events.*), created_at
              FROM security_audit_events
            ) sources
            WHERE COALESCE('household:' || household_id::text, 'global') = chain_row.chain_key
              AND NOT EXISTS (
                SELECT 1
                FROM audit_ledger_entries entries
                WHERE entries.source_table = sources.source_table AND entries.source_id = sources.source_id
              )
            ORDER BY occurred_at, source_table, source_id
          LOOP
            PERFORM audit_append_ledger_entry(
              source_row.source_table, source_row.source_id, source_row.household_id,
              source_row.source_payload, source_row.occurred_at
            );
          END LOOP;

          SELECT * INTO STRICT chain_head
          FROM audit_chain_heads
          WHERE id = chain_head.id;

          INSERT INTO audit_checkpoints (
            household_id, chain_key, chain_epoch, checkpoint_kind, sequence, entry_hash,
            created_at, updated_at
          ) VALUES (
            chain_head.household_id, chain_head.chain_key, chain_head.chain_epoch, 'legacy-repair',
            chain_head.last_sequence, chain_head.last_hash, clock_timestamp(), clock_timestamp()
          );

          UPDATE audit_chain_heads
          SET chain_epoch = gen_random_uuid(), epoch_kind = 'live',
              last_sequence = 0, last_hash = NULL, updated_at = clock_timestamp()
          WHERE id = chain_head.id;
        END LOOP;

        IF EXISTS (
          SELECT 1
          FROM (
            SELECT sources.source_table, sources.source_id
            FROM (
              SELECT 'versions'::text source_table, id source_id FROM versions
              UNION ALL
              SELECT 'security_audit_events'::text, id FROM security_audit_events
            ) sources
            LEFT JOIN audit_ledger_entries entries
              ON entries.source_table = sources.source_table AND entries.source_id = sources.source_id
            GROUP BY sources.source_table, sources.source_id
            HAVING COUNT(entries.id) <> 1
          ) incomplete_sources
        ) THEN
          RAISE EXCEPTION 'audit ledger repair did not produce exactly one entry per supported source row';
        END IF;
      END
      $$;

      DROP POLICY repair_rls_hidden_audit_sources_select ON versions;
      DROP POLICY repair_rls_hidden_audit_sources_select ON security_audit_events;
    SQL
  end

  def down
    raise ActiveRecord::IrreversibleMigration, 'Repaired audit evidence is append-only'
  end
end
