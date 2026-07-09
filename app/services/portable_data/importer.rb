# frozen_string_literal: true

module PortableData
  class Importer
    RECORD_TYPES = %w[
      people locations medications dosage_options schedules person_medications medication_takes notification_preferences
    ].freeze

    Result = Data.define(:applied, :counts, :conflicts, :errors) do
      def applied? = applied
    end

    class Error < StandardError; end

    def initialize(household:, membership:, envelope:, passphrase:, options:)
      @household = household
      @membership = membership
      @envelope = envelope
      @passphrase = passphrase
      @dry_run = options.fetch(:dry_run)
      @request = options[:request]
    end

    def call
      validate_payload!
      authorize_import!
      return result(applied: false, preflight: preflight_result) if dry_run || preflight_result.blocked?

      apply_import
    rescue ActiveRecord::RecordInvalid => e
      result(applied: false, errors: e.record.errors.full_messages)
    rescue ActiveRecord::RecordNotFound => e
      result(applied: false, errors: [e.message])
    end

    private

    attr_reader :household, :membership, :envelope, :passphrase, :dry_run, :request

    def payload
      @payload ||= Encryptor.decrypt(envelope, passphrase: passphrase).with_indifferent_access
    end

    def validate_payload!
      raise Error, 'Unsupported portable data format' unless payload[:format] == Exporter::FORMAT
      raise Error, 'Portable data records are required' unless payload[:records].is_a?(Hash)

      validate_record_collections!
    end

    def authorize_import!
      ImportAuthorization.new(household: household, membership: membership, payload: payload).call
    end

    def validate_record_collections!
      raise Error, unsupported_record_types_message if unknown_record_types.any?

      payload[:records].each do |record_type, rows|
        validate_record_collection!(record_type, rows)
      end
    end

    def unknown_record_types
      @unknown_record_types ||= payload[:records].keys.map(&:to_s) - RECORD_TYPES
    end

    def unsupported_record_types_message
      "Unsupported portable record types: #{unknown_record_types.sort.join(', ')}"
    end

    def validate_record_collection!(record_type, rows)
      raise Error, "#{record_type} must be an array" unless rows.is_a?(Array)

      rows.each_with_index do |row, index|
        validate_record_row!(record_type, row, index)
      end
    end

    def validate_record_row!(record_type, row, index)
      raise Error, "#{record_type} must contain objects" unless row.respond_to?(:to_h)

      portable_id = row.to_h.with_indifferent_access[:portable_id]
      raise Error, "#{record_type}[#{index}].portable_id is required" if portable_id.blank?
    end

    def counts
      @counts ||= payload.fetch(:records).transform_values { |rows| Array(rows).size }
    end

    def preflight_result
      @preflight_result ||= ImportPreflight.new(household: household, payload: payload).call
    end

    def apply_import
      ActiveRecord::Base.transaction do
        ImportWriter.new(household: household, membership: membership, payload: payload).call
        record_audit_event
      end

      result(applied: true)
    end

    def result(applied:, preflight: nil, errors: [])
      Result.new(
        applied: applied,
        counts: counts,
        conflicts: preflight&.conflicts || [],
        errors: preflight&.errors || errors
      )
    end

    def record_audit_event
      SecurityAuditEvent.create!(
        household: household,
        actor_account: membership.account,
        actor_membership: membership,
        event_type: 'portable_data.imported',
        request_id: request&.request_id,
        ip: request&.remote_ip,
        metadata: {
          record_counts: counts,
          dry_run: false
        }
      )
    end
  end
end
