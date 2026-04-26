# frozen_string_literal: true

module NhsDmd
  module ReleaseImportCounts
    COUNT_DEFAULTS = {
      processed: 0,
      ampp_processed: 0,
      ampp_named: 0,
      ampp_skipped: 0,
      gtin_processed: 0,
      created: 0,
      updated: 0,
      unchanged: 0,
      skipped_expired: 0,
      skipped_missing_name: 0,
      skipped_invalid: 0
    }.freeze

    private

    def build_counts(ampp_file, gtin_file)
      ampp_total = count_ampp_records(ampp_file)
      gtin_total = count_gtin_records(gtin_file)

      COUNT_DEFAULTS.merge(ampp_total:, gtin_total:, total: ampp_total + gtin_total)
    end

    def build_result(counts)
      self.class::Result.new(
        created_count: counts[:created],
        updated_count: counts[:updated],
        unchanged_count: counts[:unchanged],
        skipped_expired_count: counts[:skipped_expired],
        skipped_missing_name_count: counts[:skipped_missing_name],
        skipped_invalid_count: counts[:skipped_invalid]
      )
    end

    def breakdown_payload(counts)
      imported = counts[:created] + counts[:updated]
      skipped = counts[:skipped_expired] + counts[:skipped_missing_name] + counts[:skipped_invalid]

      {
        imported_count: imported,
        skipped_count: skipped,
        created_count: counts[:created],
        updated_count: counts[:updated],
        unchanged_count: counts[:unchanged],
        skipped_expired_count: counts[:skipped_expired],
        skipped_missing_name_count: counts[:skipped_missing_name],
        skipped_invalid_count: counts[:skipped_invalid]
      }
    end

    def skipped_total(counts)
      counts[:skipped_expired] + counts[:skipped_missing_name] + counts[:skipped_invalid]
    end
  end
end
