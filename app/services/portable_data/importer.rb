# frozen_string_literal: true

module PortableData
  class Importer
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
    end

    def authorize_import!
      return if membership&.owner? || membership&.administrator?

      payload_person_ids = records(:people).pluck(:portable_id)
      manageable_ids = manageable_payload_person_ids(payload_person_ids)
      return if payload_person_ids.present? && (payload_person_ids - manageable_ids).empty?

      raise Pundit::NotAuthorizedError
    end

    def manageable_payload_person_ids(payload_person_ids)
      Person.joins(:person_access_grants)
            .where(household: household, portable_id: payload_person_ids)
            .where(person_access_grants: manage_grant_conditions)
            .pluck(:portable_id)
    end

    def manage_grant_conditions
      {
        household: household,
        household_membership: membership,
        access_level: :manage,
        revoked_at: nil
      }
    end

    def records(name)
      Array(payload.dig(:records, name)).map(&:with_indifferent_access)
    end

    def counts
      @counts ||= payload.fetch(:records).transform_values { |rows| Array(rows).size }
    end

    def preflight_result
      @preflight_result ||= ImportPreflight.new(household: household, payload: payload).call
    end

    def apply_import
      ActiveRecord::Base.transaction do
        ImportWriter.new(household: household, payload: payload).call
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
