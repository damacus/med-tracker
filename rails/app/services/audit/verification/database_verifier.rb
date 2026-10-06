# frozen_string_literal: true

require 'digest'
require 'json'
require 'openssl'

module Audit
  module Verification
    class DatabaseVerifier
      SOURCE_TABLES = %w[security_audit_events versions].freeze
      HASH_DOMAIN = 'medtracker.audit.ledger.v1'
      CHECKPOINT_DOMAIN = 'medtracker.audit.checkpoint.v1'
      SEPARATOR = "\u001F"

      def initialize(entries: AuditLedgerEntry.all, verify_heads: true, household_id: nil)
        @entries_source = entries
        @verify_heads = verify_heads
        @household_id = household_id
        @issues = []
      end

      def call
        DatabaseAuthority.new.verify!
        verify_entries
        issues.concat(SourceCompletenessVerifier.new(household_id:).call)
        verify_chain_heads if verify_heads
        checked_checkpoints = verify_checkpoints
        Result.new(
          scope: 'database', checked_entries: entries.size, checked_checkpoints:,
          checked_objects: 0, issues: sorted_issues
        )
      end

      private

      attr_reader :entries_source, :household_id, :issues, :verify_heads

      def entries
        @entries ||= scoped_records(entries_source)
                     .sort_by { |entry| [entry.chain_key, entry.chain_epoch, entry.sequence] }
      end

      def verify_entries
        entries.group_by { |entry| [entry.chain_key, entry.chain_epoch] }.each_value do |chain_entries|
          verify_chain_entries(chain_entries)
        end
      end

      def verify_chain_entries(chain_entries)
        chain_entries.each_with_index do |entry, index|
          predecessor = index.zero? ? stored_predecessor(entry) : chain_entries[index - 1]
          verify_sequence(entry, predecessor)
          verify_previous_hash(entry, predecessor)
          verify_canonical_payload(entry)
          verify_entry_hash(entry)
          verify_source_payload(entry)
        end
      end

      def stored_predecessor(entry)
        return if entry.sequence == 1

        scoped_relation(AuditLedgerEntry.all).find_by(
          chain_key: entry.chain_key, chain_epoch: entry.chain_epoch, sequence: entry.sequence - 1
        )
      end

      def verify_sequence(entry, predecessor)
        expected = predecessor ? predecessor.sequence + 1 : 1
        add_issue('sequence_gap', 'ledger sequence is missing or duplicated', entry) unless entry.sequence == expected
      end

      def verify_previous_hash(entry, predecessor)
        expected = predecessor&.entry_hash
        return if entry.previous_hash == expected

        add_issue('previous_hash_mismatch', 'ledger previous hash does not match',
                  entry)
      end

      def verify_canonical_payload(entry)
        parsed = JSON.parse(entry.canonical_payload)
        unless parsed == entry.envelope
          add_issue('canonical_payload_mismatch', 'canonical payload does not match its envelope',
                    entry)
        end
      rescue JSON::ParserError
        add_issue('canonical_payload_invalid', 'canonical payload is not valid JSON', entry)
      end

      def verify_entry_hash(entry)
        calculated = Digest::SHA256.digest(hash_input(entry))
        return if calculated == entry.entry_hash

        add_issue('entry_hash_mismatch', 'ledger entry hash does not match its payload',
                  entry)
      end

      def hash_input(entry)
        prefix = [HASH_DOMAIN, entry.chain_key, entry.chain_epoch, entry.sequence].join(SEPARATOR) + SEPARATOR
        prefix.b + (entry.previous_hash || ''.b) + entry.canonical_payload
      end

      def verify_source_payload(entry)
        unless SOURCE_TABLES.include?(entry.source_table)
          add_issue('source_table_unknown', 'ledger source table is not supported', entry)
          return
        end

        current_payload = SourcePayloadReader.new.call(entry)
        code = current_payload ? 'source_payload_mismatch' : 'source_row_missing'
        message = current_payload ? 'source row no longer matches the ledger' : 'source row is missing'
        add_issue(code, message, entry) unless current_payload == entry.source_payload
      end

      def verify_chain_heads
        relevant_chain_heads.each { |head| verify_chain_head(head) }
      end

      def relevant_chain_heads
        heads = scoped_relation(AuditChainHead.all).to_a
        return heads if entries.empty?

        chain_keys = entries.map(&:chain_key).uniq
        heads.select { |head| chain_keys.include?(head.chain_key) }
      end

      def verify_chain_head(head)
        tail = chain_tail(head)
        return if head_matches?(head, tail)

        add_issue('chain_head_mismatch', 'chain head does not match the retained tail', tail, head.chain_key,
                  head.last_sequence)
      end

      def chain_tail(head)
        entries.select do |entry|
          entry.chain_key == head.chain_key && entry.chain_epoch == head.chain_epoch
        end.max_by(&:sequence)
      end

      def head_matches?(head, tail)
        return head.last_sequence.zero? && head.last_hash.nil? unless tail

        head.last_sequence == tail.sequence && head.last_hash == tail.entry_hash
      end

      def verify_checkpoints
        checkpoints = checkpoints_for_entries
        checkpoints.each { |checkpoint| verify_checkpoint(checkpoint) }
        checkpoints.size
      end

      def checkpoints_for_entries
        keys = entries.to_h { |entry| [[entry.chain_key, entry.chain_epoch, entry.sequence], true] }
        scoped_relation(AuditCheckpoint.includes(:audit_signing_key)).select do |checkpoint|
          keys.key?([checkpoint.chain_key, checkpoint.chain_epoch, checkpoint.sequence])
        end
      end

      def scoped_records(source)
        return source.to_a unless household_id
        return source.where(household_id:).to_a if source.respond_to?(:where)

        source.to_a.select { |record| record.household_id == household_id }
      end

      def scoped_relation(relation)
        household_id ? relation.where(household_id:) : relation
      end

      def verify_checkpoint(checkpoint)
        verify_checkpoint_entry(checkpoint)
        return unless checkpoint_signed?(checkpoint)

        verify_checkpoint_signature(checkpoint)
      end

      def verify_checkpoint_entry(checkpoint)
        entry = checkpoint_entry(checkpoint)
        return if entry&.entry_hash == checkpoint.entry_hash

        add_checkpoint_issue('checkpoint_entry_mismatch', 'checkpoint does not identify a retained entry', checkpoint)
      end

      def checkpoint_entry(checkpoint)
        entries.find do |candidate|
          candidate.chain_key == checkpoint.chain_key && candidate.chain_epoch == checkpoint.chain_epoch &&
            candidate.sequence == checkpoint.sequence
        end
      end

      def checkpoint_signed?(checkpoint)
        return true if checkpoint.signature.present? && checkpoint.audit_signing_key && checkpoint.signed_at

        add_checkpoint_issue('checkpoint_unsigned', 'checkpoint has no verifiable signature', checkpoint)
        false
      end

      def verify_checkpoint_signature(checkpoint)
        public_key = OpenSSL::PKey.read(checkpoint.audit_signing_key.public_key)
        return if public_key.verify(nil, checkpoint.signature, checkpoint_payload(checkpoint))

        add_checkpoint_issue('checkpoint_signature_invalid', 'checkpoint signature is invalid', checkpoint)
      rescue OpenSSL::PKey::PKeyError
        add_checkpoint_issue('checkpoint_signature_invalid', 'checkpoint signature is invalid', checkpoint)
      end

      def checkpoint_payload(checkpoint)
        [
          CHECKPOINT_DOMAIN, checkpoint.audit_signing_key.key_id, checkpoint.chain_key,
          checkpoint.chain_epoch, checkpoint.sequence, checkpoint.entry_hash.unpack1('H*'),
          checkpoint.signed_at.utc.iso8601(6)
        ].join(SEPARATOR)
      end

      def add_checkpoint_issue(code, message, checkpoint)
        issues << Issue.new(code:, message:, chain_key: checkpoint.chain_key, sequence: checkpoint.sequence)
      end

      def add_issue(code, message, entry = nil, chain_key = nil, sequence = nil)
        issues << Issue.new(
          code:, message:, chain_key: chain_key || entry&.chain_key, sequence: sequence || entry&.sequence
        )
      end

      def sorted_issues
        issues.sort_by { |issue| [issue.chain_key.to_s, issue.sequence.to_i, issue.code] }
      end
    end
  end
end
