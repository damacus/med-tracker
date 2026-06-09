# frozen_string_literal: true

require 'rails_helper'

# Spec-only host that includes both mixins so all inter-mixin calls resolve.
module NhsDmd
  class ReleaseImportProgressSpecHost
    include ReleaseImportCounts
    include ReleaseImportProgress

    public :emit_progress,
           :progress_payload,
           :emit_initial_progress,
           :process_unmatched_gtins,
           :progress_batch_reached?,
           :counting_progress,
           :starting_ampp_progress,
           :emit_gtin_start_progress,
           :initial_progress_payload,
           :track_ampp_progress,
           :mark_gtin_processed,
           :ampp_progress_message,
           :gtin_progress_message

    private

    # These abstract methods are implemented by the real class.
    def count_ampp_records(_file) = 10
    def count_gtin_records(_file) = 5
  end
end

RSpec.describe NhsDmd::ReleaseImportProgress do
  subject(:host) { NhsDmd::ReleaseImportProgressSpecHost.new }

  let(:callback) { instance_double(Proc, call: nil) }
  let(:base_counts) do
    NhsDmd::ReleaseImportCounts::COUNT_DEFAULTS.merge(
      ampp_total: 100,
      gtin_total: 50,
      total: 150
    )
  end

  describe 'PROGRESS_BATCH_SIZE' do
    it 'is 250' do
      expect(described_class::PROGRESS_BATCH_SIZE).to eq(250)
    end
  end

  describe '#progress_batch_reached?' do
    it 'returns false when processed is zero' do
      counts = base_counts.merge(processed: 0)

      expect(host.progress_batch_reached?(counts)).to be false
    end

    it 'returns true when processed is a multiple of PROGRESS_BATCH_SIZE' do
      counts = base_counts.merge(processed: 250)

      expect(host.progress_batch_reached?(counts)).to be true
    end

    it 'returns false when processed is not a multiple of PROGRESS_BATCH_SIZE' do
      counts = base_counts.merge(processed: 251)

      expect(host.progress_batch_reached?(counts)).to be false
    end
  end

  describe '#emit_progress' do
    context 'when there is no callback' do
      it 'does nothing' do
        host.emit_progress(base_counts, nil, message: 'test')

        # No error means success — no callback to verify
        expect(true).to be true
      end
    end

    context 'when the batch boundary is not reached' do
      it 'does not call the callback' do
        counts = base_counts.merge(processed: 1)

        host.emit_progress(counts, callback, message: 'test')

        expect(callback).not_to have_received(:call)
      end
    end

    context 'when the batch boundary is reached' do
      it 'calls the callback with the progress payload' do
        counts = base_counts.merge(processed: 250)

        host.emit_progress(counts, callback, message: 'Batch done')

        expect(callback).to have_received(:call).with(
          hash_including(status: :importing, message: 'Batch done', processed_records: 250)
        )
      end
    end

    context 'when force: true' do
      it 'calls the callback even when the batch boundary is not reached' do
        counts = base_counts.merge(processed: 1)

        host.emit_progress(counts, callback, message: 'forced', force: true)

        expect(callback).to have_received(:call)
      end
    end
  end

  describe '#progress_payload' do
    subject(:payload) { host.progress_payload(base_counts.merge(processed: 50), 'Processing') }

    it 'includes status :importing' do
      expect(payload[:status]).to eq(:importing)
    end

    it 'includes the message' do
      expect(payload[:message]).to eq('Processing')
    end

    it 'includes total and processed record counts' do
      expect(payload[:total_records]).to eq(150)
      expect(payload[:processed_records]).to eq(50)
    end

    it 'includes all breakdown fields' do
      expect(payload).to include(
        :imported_count, :skipped_count, :created_count, :updated_count,
        :unchanged_count, :skipped_expired_count, :skipped_missing_name_count,
        :skipped_invalid_count
      )
    end
  end

  describe '#initial_progress_payload' do
    subject(:payload) do
      host.initial_progress_payload(status: :counting, message: 'Counting', total_records: 150)
    end

    it 'sets all count fields to zero' do
      count_keys = %i[
        processed_records imported_count skipped_count created_count
        updated_count unchanged_count skipped_expired_count
        skipped_missing_name_count skipped_invalid_count
      ]

      count_keys.each do |key|
        expect(payload[key]).to eq(0), "expected payload[#{key}] to be 0"
      end
    end

    it 'passes through status, message, and total_records' do
      expect(payload).to include(status: :counting, message: 'Counting', total_records: 150)
    end
  end

  describe '#counting_progress' do
    it 'returns a payload with status :counting and the record counts in the message' do
      counts = base_counts.merge(ampp_total: 100, gtin_total: 50, total: 150)
      payload = host.counting_progress(counts)

      expect(payload[:status]).to eq(:counting)
      expect(payload[:message]).to include('100 AMPP records', '50 GTIN records')
    end
  end

  describe '#starting_ampp_progress' do
    it 'returns a payload with the starting AMPP import message' do
      payload = host.starting_ampp_progress(base_counts)

      expect(payload[:status]).to eq(:importing)
      expect(payload[:message]).to eq('Starting AMPP name import')
    end
  end

  describe '#emit_initial_progress' do
    it 'calls the callback twice — once for counting, once for starting AMPP' do
      host.emit_initial_progress(callback, base_counts)

      expect(callback).to have_received(:call).twice
    end

    it 'does nothing when there is no callback' do
      expect { host.emit_initial_progress(nil, base_counts) }.not_to raise_error
    end
  end

  describe '#emit_gtin_start_progress' do
    it 'calls the callback with a starting GTIN payload' do
      counts = base_counts.merge(processed: 100)

      host.emit_gtin_start_progress(callback, counts)

      expect(callback).to have_received(:call).with(
        hash_including(status: :importing, message: 'Starting GTIN import', processed_records: 100)
      )
    end

    it 'does nothing when there is no callback' do
      expect { host.emit_gtin_start_progress(nil, base_counts) }.not_to raise_error
    end
  end

  describe '#track_ampp_progress' do
    let(:counts) { base_counts.dup }

    it 'increments ampp_processed and processed' do
      host.track_ampp_progress(counts, nil)

      expect(counts[:ampp_processed]).to eq(1)
      expect(counts[:processed]).to eq(1)
    end
  end

  describe '#mark_gtin_processed' do
    let(:counts) { base_counts.dup }

    it 'increments gtin_processed and processed' do
      host.mark_gtin_processed(counts)

      expect(counts[:gtin_processed]).to eq(1)
      expect(counts[:processed]).to eq(1)
    end
  end

  describe '#ampp_progress_message' do
    it 'includes ampp_processed, ampp_named, and ampp_skipped counts' do
      counts = base_counts.merge(ampp_processed: 5, ampp_named: 3, ampp_skipped: 2)

      message = host.ampp_progress_message(counts)

      expect(message).to include('5 AMPP records', '3 updated', '2 skipped')
    end
  end

  describe '#gtin_progress_message' do
    it 'includes overall processed count plus per-outcome breakdown' do
      counts = base_counts.merge(
        processed: 20, created: 10, updated: 5, unchanged: 3,
        skipped_expired: 1, skipped_missing_name: 1, skipped_invalid: 0
      )

      message = host.gtin_progress_message(counts)

      expect(message).to include('20 records', '10 new', '5 updated', '3 unchanged', '2 skipped')
    end
  end
end
