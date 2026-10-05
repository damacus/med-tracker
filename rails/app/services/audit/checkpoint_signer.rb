# frozen_string_literal: true

require 'base64'
require 'openssl'

module Audit
  class CheckpointSigner
    class AlreadySigned < StandardError; end

    CHECKPOINT_SQL = <<~SQL.squish.freeze
      SELECT audit_record_signed_checkpoint(
        $1, $2, $3, $4::uuid, $5::bigint, $6, $7, $8::timestamptz, $9
      )
    SQL

    def initialize(key_id:, private_key_pem:)
      @key_id = key_id
      @private_key = OpenSSL::PKey.read(private_key_pem)
      raise ArgumentError, 'checkpoint signing key must be Ed25519' unless private_key.oid == 'ED25519'
    end

    def sign(entry)
      ensure_unsigned!(entry)
      signed_at = Time.current.utc
      signature = private_key.sign(nil, payload_for(entry, signed_at:))
      checkpoint_id = record_checkpoint(entry, signed_at:, signature:)
      AuditCheckpoint.find(checkpoint_id)
    rescue ActiveRecord::StatementInvalid => e
      raise AlreadySigned, 'checkpoint is already signed by another key' if e.message.include?('already signed')

      raise
    end

    def payload_for(entry, signed_at: nil)
      checkpoint = AuditCheckpoint.find_by(chain_key: entry.chain_key, chain_epoch: entry.chain_epoch,
                                           sequence: entry.sequence)
      signed_at ||= checkpoint&.signed_at
      [
        'medtracker.audit.checkpoint.v1', key_id, entry.chain_key, entry.chain_epoch,
        entry.sequence, entry.entry_hash.unpack1('H*'), signed_at&.utc&.iso8601(6)
      ].join("\u001F")
    end

    private

    attr_reader :key_id, :private_key

    def ensure_unsigned!(entry)
      existing = AuditCheckpoint.find_by(chain_key: entry.chain_key, chain_epoch: entry.chain_epoch,
                                         sequence: entry.sequence)
      raise AlreadySigned, 'checkpoint is already signed by another key' if existing&.signature.present?
    end

    def record_checkpoint(entry, signed_at:, signature:)
      ActiveRecord::Base.connection
                        .select_value(CHECKPOINT_SQL, 'Audit checkpoint', checkpoint_binds(entry, signed_at, signature))
                        .to_i
    end

    def checkpoint_binds(entry, signed_at, signature)
      checkpoint_values(entry, signed_at, signature).map.with_index do |value, index|
        ActiveRecord::Relation::QueryAttribute.new("audit_checkpoint_#{index}", value, ActiveRecord::Type::String.new)
      end
    end

    def checkpoint_values(entry, signed_at, signature)
      [
        key_id, Base64.strict_encode64(private_key.public_to_der), entry.chain_key, entry.chain_epoch,
        entry.sequence, entry.entry_hash.unpack1('H*'), Base64.strict_encode64(signature),
        signed_at.iso8601(6), 'periodic'
      ]
    end
  end
end
