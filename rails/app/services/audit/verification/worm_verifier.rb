# frozen_string_literal: true

module Audit
  module Verification
    class WormVerifier
      def initialize(adapter:, records: nil, deliveries: AuditExportDelivery.all)
        @records = records || default_records
        @deliveries = deliveries
        @adapter = adapter
        @issues = []
        @checked_objects = 0
      end

      def call
        delivery_by_record = deliveries.to_a.index_by(&:export_record)
        records.to_a.each { |record| verify_record(record, delivery_by_record[record]) }
        Result.new(
          scope: 'worm', checked_entries: 0, checked_checkpoints: 0,
          checked_objects:, issues: sorted_issues
        )
      end

      private

      attr_reader :records, :deliveries, :adapter, :issues, :checked_objects

      def default_records
        AuditLedgerEntry.all.to_a + AuditCheckpoint.where.not(signature: nil).to_a
      end

      def verify_record(record, delivery)
        unless delivery
          add_issue('worm_delivery_missing', 'audit record has no delivery receipt', record)
          return
        end
        unless delivery.delivered?
          code = delivery.failed? ? 'worm_delivery_failed' : 'worm_delivery_pending'
          add_issue(code, 'audit record has not been retained in Object Lock', record)
          return
        end

        adapter.verify(record, delivery:)
        @checked_objects += 1
      rescue Audit::ObjectLock::IntegrityError => e
        add_issue('worm_object_invalid', e.message, record)
      end

      def add_issue(code, message, record)
        issues << Issue.new(code:, message:, chain_key: record.chain_key, sequence: record.sequence)
      end

      def sorted_issues
        issues.sort_by { |issue| [issue.chain_key.to_s, issue.sequence.to_i, issue.code] }
      end
    end
  end
end
