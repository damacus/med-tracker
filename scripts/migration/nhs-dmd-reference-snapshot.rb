result = NhsDmd::ReleaseArchiveImport.new.import(ENV.fetch('NHS_DMD_RELEASE_PATH'))
connection = ActiveRecord::Base.connection
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
    'trade_memberships', (SELECT count(*) FROM nhs_dmd_amp_trade_families)
  )::text
SQL
File.write(ENV.fetch('NHS_DMD_COMPARISON_OUTPUT'), rows)
