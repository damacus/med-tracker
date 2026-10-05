# frozen_string_literal: true

module Audit
  module ObjectLock
    class Runner
      DEFAULT_INTERVAL = 15
      DEFAULT_BATCH_SIZE = 100

      def self.from_environment(environment = ENV)
        configuration = Configuration.new(environment)
        adapter = S3Adapter.new(configuration:)
        signer = Audit::CheckpointSigner.new(
          key_id: environment.fetch('AUDIT_SIGNING_KEY_ID'),
          private_key_pem: signing_key(environment)
        )
        new(adapter:, signer:, delivery_exporter: DeliveryExporter.new(adapter:))
      end

      def self.signing_key(environment)
        if environment['AUDIT_SIGNING_PRIVATE_KEY_FILE'].present?
          return File.binread(environment['AUDIT_SIGNING_PRIVATE_KEY_FILE'])
        end

        environment.fetch('AUDIT_SIGNING_PRIVATE_KEY')
      end

      def initialize(adapter:, signer:, delivery_exporter:, **options)
        @adapter = adapter
        @signer = signer
        @delivery_exporter = delivery_exporter
        @checkpoint_entries = options[:checkpoint_entries] || method(:default_checkpoint_entries)
        @pending_deliveries = options[:pending_deliveries] || method(:default_pending_deliveries)
        @clock = options[:clock] || -> { Process.clock_gettime(Process::CLOCK_MONOTONIC) }
        @validation_interval = options.fetch(:validation_interval, 300)
        @stopped = false
      end

      def startup!
        adapter.validate!
        @last_validation_at = clock.call
      end

      def run_once
        validate_if_due
        checkpoint_entries.call.each { |entry| sign_unless_completed(entry) }
        pending_deliveries.call.each { |delivery| delivery_exporter.deliver(delivery) }
      end

      def run_forever(interval: ENV.fetch('AUDIT_EXPORT_INTERVAL_SECONDS', DEFAULT_INTERVAL).to_i)
        startup!
        until stopped?
          run_once
          sleep interval unless stopped?
        end
      end

      def stop!
        @stopped = true
      end

      private

      attr_reader :adapter, :signer, :delivery_exporter, :checkpoint_entries, :pending_deliveries,
                  :clock, :validation_interval

      def stopped?
        @stopped
      end

      def validate_if_due
        return if @last_validation_at && clock.call - @last_validation_at < validation_interval

        startup!
      end

      def sign_unless_completed(entry)
        signer.sign(entry)
      rescue Audit::CheckpointSigner::AlreadySigned
        nil
      end

      def default_checkpoint_entries
        (unsigned_checkpoint_entries + current_chain_entries).uniq(&:id)
      end

      def unsigned_checkpoint_entries
        AuditCheckpoint.where(signature: nil).filter_map do |checkpoint|
          AuditLedgerEntry.find_by(
            chain_key: checkpoint.chain_key, chain_epoch: checkpoint.chain_epoch,
            sequence: checkpoint.sequence, entry_hash: checkpoint.entry_hash
          )
        end
      end

      def current_chain_entries
        AuditChainHead.where('last_sequence > 0').filter_map do |head|
          AuditLedgerEntry.find_by(
            chain_key: head.chain_key, chain_epoch: head.chain_epoch,
            sequence: head.last_sequence, entry_hash: head.last_hash
          )
        end
      end

      def default_pending_deliveries
        AuditExportDelivery.pending
                           .where('next_attempt_at IS NULL OR next_attempt_at <= ?', Time.current)
                           .order(:created_at)
                           .limit(DEFAULT_BATCH_SIZE)
      end
    end
  end
end
