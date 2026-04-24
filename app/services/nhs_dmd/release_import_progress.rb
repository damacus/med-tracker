# frozen_string_literal: true

module NhsDmd
  module ReleaseImportProgress
    PROGRESS_BATCH_SIZE = 250

    private

    def emit_progress(counts, progress_callback, message:, force: false)
      return unless progress_callback
      return unless force || progress_batch_reached?(counts)

      progress_callback.call(progress_payload(counts, message))
    end

    def progress_payload(counts, message)
      {
        status: :importing,
        message: message,
        total_records: counts[:total],
        processed_records: counts[:processed]
      }.merge(breakdown_payload(counts))
    end

    def emit_initial_progress(progress_callback, counts)
      return unless progress_callback

      progress_callback.call(counting_progress(counts))
      progress_callback.call(starting_ampp_progress(counts))
    end

    def process_unmatched_gtins(doc, counts, progress_callback)
      doc.css('GTINDATA').each do
        mark_gtin_processed(counts)
        counts[:skipped_missing_name] += 1
        emit_progress(counts, progress_callback, message: gtin_progress_message(counts))
      end
    end

    def progress_batch_reached?(counts)
      counts[:processed].positive? && (counts[:processed] % PROGRESS_BATCH_SIZE).zero?
    end

    def counting_progress(counts)
      initial_progress_payload(
        status: :counting,
        message: "Counted #{counts[:ampp_total]} AMPP records and #{counts[:gtin_total]} GTIN records",
        total_records: counts[:total]
      )
    end

    def starting_ampp_progress(counts)
      initial_progress_payload(
        status: :importing,
        message: 'Starting AMPP name import',
        total_records: counts[:total]
      )
    end

    def emit_gtin_start_progress(progress_callback, counts)
      return unless progress_callback

      progress_callback.call(
        initial_progress_payload(
          status: :importing,
          message: 'Starting GTIN import',
          total_records: counts[:total]
        ).merge(processed_records: counts[:processed])
      )
    end

    def initial_progress_payload(status:, message:, total_records:)
      {
        status: status,
        message: message,
        total_records: total_records,
        processed_records: 0,
        imported_count: 0,
        skipped_count: 0,
        created_count: 0,
        updated_count: 0,
        unchanged_count: 0,
        skipped_expired_count: 0,
        skipped_missing_name_count: 0,
        skipped_invalid_count: 0
      }
    end

    def track_ampp_progress(counts, progress_callback)
      counts[:ampp_processed] += 1
      counts[:processed] += 1
      emit_progress(counts, progress_callback, message: ampp_progress_message(counts))
    end

    def mark_gtin_processed(counts)
      counts[:gtin_processed] += 1
      counts[:processed] += 1
    end

    def ampp_progress_message(counts)
      "Processed #{counts[:ampp_processed]} AMPP records " \
        "(#{counts[:ampp_named]} updated, #{counts[:ampp_skipped]} skipped)"
    end

    def gtin_progress_message(counts)
      "Processed #{counts[:processed]} records " \
        "(#{counts[:created]} new, #{counts[:updated]} updated, " \
        "#{counts[:unchanged]} unchanged, #{skipped_total(counts)} skipped)"
    end
  end
end
