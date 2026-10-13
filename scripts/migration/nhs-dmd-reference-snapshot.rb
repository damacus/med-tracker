snapshot_started = Process.clock_gettime(Process::CLOCK_MONOTONIC)
connection = ActiveRecord::Base.connection
initial_count = connection.select_value(<<~SQL)
  SELECT (SELECT count(*) FROM nhs_dmd_barcodes)
       + (SELECT count(*) FROM nhs_dmd_ampp_relationships)
       + (SELECT count(*) FROM nhs_dmd_trade_families)
       + (SELECT count(*) FROM nhs_dmd_trade_family_groups)
       + (SELECT count(*) FROM nhs_dmd_amp_trade_families)
       + (SELECT count(*) FROM nhs_dmd_supplementary_releases)
SQL
raise 'Rails reference catalogue must start empty' unless initial_count.to_i.zero?

import_started = Process.clock_gettime(Process::CLOCK_MONOTONIC)
result = NhsDmd::ReleaseArchiveImport.new.import(ENV.fetch('NHS_DMD_RELEASE_PATH'))
import_finished = Process.clock_gettime(Process::CLOCK_MONOTONIC)
rows = connection.select_value(<<~SQL)
  SELECT jsonb_build_object(
    'status', 4,
    'created', #{Integer(result.created_count)},
    'updated', #{Integer(result.updated_count)},
    'unchanged', #{Integer(result.unchanged_count)},
    'skipped_expired', #{Integer(result.skipped_expired_count)},
    'skipped_missing_name', #{Integer(result.skipped_missing_name_count)},
    'skipped_invalid', #{Integer(result.skipped_invalid_count)},
    'barcodes', (SELECT COALESCE(jsonb_agg(jsonb_build_array(gtin, code, amp_code, display, vmp_name, system, concept_class) ORDER BY gtin), '[]'::jsonb) FROM nhs_dmd_barcodes),
    'relationships', (SELECT COALESCE(jsonb_agg(jsonb_build_array(ampp_code, amp_code) ORDER BY ampp_code), '[]'::jsonb) FROM nhs_dmd_ampp_relationships),
    'trade_families', (SELECT count(*) FROM nhs_dmd_trade_families),
    'trade_family_groups', (SELECT count(*) FROM nhs_dmd_trade_family_groups),
    'trade_memberships', (SELECT count(*) FROM nhs_dmd_amp_trade_families),
    'trade_family_group_rows', (SELECT COALESCE(jsonb_agg(jsonb_build_array(code, name) ORDER BY code), '[]'::jsonb) FROM nhs_dmd_trade_family_groups),
    'trade_family_rows', (SELECT COALESCE(jsonb_agg(jsonb_build_array(f.code, f.name, g.code) ORDER BY f.code), '[]'::jsonb) FROM nhs_dmd_trade_families f LEFT JOIN nhs_dmd_trade_family_groups g ON g.id = f.trade_family_group_id),
    'trade_membership_rows', (SELECT COALESCE(jsonb_agg(jsonb_build_array(m.amp_code, f.code) ORDER BY m.amp_code), '[]'::jsonb) FROM nhs_dmd_amp_trade_families m JOIN nhs_dmd_trade_families f ON f.id = m.trade_family_id),
    'supplementary_release_rows', (SELECT COALESCE(jsonb_agg(jsonb_build_array(released_on) ORDER BY released_on), '[]'::jsonb) FROM nhs_dmd_supplementary_releases)
  )::text
SQL
File.write(ENV.fetch('NHS_DMD_COMPARISON_OUTPUT'), rows)
snapshot_finished = Process.clock_gettime(Process::CLOCK_MONOTONIC)
if ENV['NHS_DMD_TIMING_OUTPUT']
  timings = {
    runtime: 'rails', outcome: 'success', initial_catalogue_count: initial_count.to_i,
    setup_ms: (import_started - snapshot_started) * 1000,
    import_ms: (import_finished - import_started) * 1000,
    export_ms: (snapshot_finished - import_finished) * 1000,
    total_ms: (snapshot_finished - snapshot_started) * 1000
  }
  File.write(ENV.fetch('NHS_DMD_TIMING_OUTPUT'), JSON.generate(timings))
end
