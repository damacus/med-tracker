# frozen_string_literal: true

require 'base64'
require 'digest'
require 'json'

module Audit
  module ObjectLock
    class RecordSerializer
      attr_reader :record

      def initialize(record)
        @record = record
      end

      def body
        @body ||= JSON.generate(deep_sort(payload))
      end

      def object_key
        record.is_a?(AuditLedgerEntry) ? ledger_object_key : checkpoint_object_key
      end

      def checksum_sha256
        @checksum_sha256 ||= Digest::SHA256.hexdigest(body)
      end

      def checksum_sha256_base64
        Base64.strict_encode64([checksum_sha256].pack('H*'))
      end

      def retain_until
        return record.retain_until if record.is_a?(AuditLedgerEntry)

        checkpoint_entry.retain_until
      end

      private

      def payload
        record.is_a?(AuditLedgerEntry) ? ledger_payload : checkpoint_payload
      end

      def ledger_payload
        {
          evidence_type: 'ledger_entry', export_schema_version: 1,
          chain: chain_payload,
          hash: ledger_hash_payload,
          source: source_payload,
          envelope: record.envelope,
          canonical_payload: Base64.strict_encode64(record.canonical_payload),
          occurred_at: timestamp(record.occurred_at),
          retention: retention_payload
        }
      end

      def checkpoint_payload
        {
          evidence_type: 'signed_checkpoint', export_schema_version: 1,
          chain: chain_payload,
          checkpoint_kind: record.checkpoint_kind,
          entry_hash: hex(record.entry_hash),
          signed_at: timestamp(record.signed_at),
          signing_key: signing_key_payload,
          signature: Base64.strict_encode64(record.signature)
        }
      end

      def ledger_hash_payload
        {
          algorithm: record.hash_algorithm,
          previous: hex(record.previous_hash),
          value: hex(record.entry_hash)
        }
      end

      def source_payload
        { table: record.source_table, id: record.source_id, payload: record.source_payload }
      end

      def retention_payload
        { policy_version: record.retention_policy_version, retain_until: timestamp(record.retain_until) }
      end

      def signing_key_payload
        key = record.audit_signing_key
        { key_id: key.key_id, algorithm: key.algorithm, public_key: Base64.strict_encode64(key.public_key) }
      end

      def chain_payload
        {
          key: record.chain_key, epoch: record.chain_epoch, sequence: record.sequence,
          epoch_kind: record.try(:epoch_kind)
        }.compact
      end

      def ledger_object_key
        object_key_for('audit-ledger')
      end

      def checkpoint_object_key
        object_key_for('audit-checkpoints')
      end

      def object_key_for(prefix)
        filename = "#{format('%020d', record.sequence)}-#{hex(record.entry_hash)}.json"
        [prefix, 'v1', chain_path, record.chain_epoch, filename].join('/')
      end

      def chain_path
        record.chain_key.gsub(/[^a-zA-Z0-9._-]/, '/')
      end

      def checkpoint_entry
        @checkpoint_entry ||= AuditLedgerEntry.find_by!(
          chain_key: record.chain_key, chain_epoch: record.chain_epoch,
          sequence: record.sequence, entry_hash: record.entry_hash
        )
      end

      def timestamp(value)
        value&.utc&.iso8601(6)
      end

      def hex(value)
        value&.unpack1('H*')
      end

      def deep_sort(value)
        case value
        when Hash then value.to_h { |key, child| [key.to_s, deep_sort(child)] }.sort.to_h
        when Array then value.map { |child| deep_sort(child) }
        else value
        end
      end
    end
  end
end
