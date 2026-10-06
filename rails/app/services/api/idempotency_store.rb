# frozen_string_literal: true

module Api
  class IdempotencyStore
    EXPIRY = 24.hours
    LOCK_NAMESPACE = 'med_tracker.api_idempotency.v1'
    REPLAYABLE_RESPONSE_HEADERS = %w[ETag].freeze

    Result = Data.define(:record, :replayed, :conflict, :response_headers)

    def initialize(request:, credential:, household:)
      @request = request
      @credential = credential
      @household = household
    end

    def active?
      key.present? && mutating_request? && household.present? && credential.present?
    end

    def with_reservation(response:)
      return yield unless active?

      result = nil
      ApiIdempotencyKey.transaction(requires_new: true) do
        acquire_reservation
        discard_expired_record
        result = lookup
        next if result.record

        yield
        persist_response!(response)
      end
      result
    end

    def lookup
      record = ApiIdempotencyKey.find_by(household: household, key: key)
      return Result.new(record: nil, replayed: false, conflict: false, response_headers: {}) unless record

      replayed = same_request?(record)
      Result.new(
        record: record,
        replayed: replayed,
        conflict: !replayed,
        response_headers: replayed ? replayable_response_headers(record.response_headers) : {}
      )
    end

    def store!(response)
      return unless active? && response.status < 500

      ApiIdempotencyKey.create!(idempotency_attributes(response))
    end

    private

    attr_reader :request, :credential, :household

    def acquire_reservation
      ApiIdempotencyKey.connection.execute(
        ActiveRecord::Base.sanitize_sql_array(
          ['SELECT pg_advisory_xact_lock(?)', reservation_id]
        )
      )
    end

    def reservation_id
      Digest::SHA256.digest(
        [LOCK_NAMESPACE, household.id, key].join("\0")
      ).unpack1('q>')
    end

    def discard_expired_record
      ApiIdempotencyKey.where(household: household, key: key, expires_at: ..Time.current).delete_all
    end

    def persist_response!(response)
      raise ActiveRecord::Rollback if response.status >= 500
      return if response.status == 409

      store!(response)
    end

    def idempotency_attributes(response)
      credential_attributes.merge(request_attributes).merge(response_attributes(response)).merge(
        household: household,
        account: credential.account,
        key: key,
        expires_at: EXPIRY.from_now
      )
    end

    def credential_attributes
      {
        api_session: credential.is_a?(ApiSession) ? credential : nil,
        api_app_token: credential.is_a?(ApiAppToken) ? credential : nil
      }
    end

    def request_attributes
      {
        request_method: request.request_method,
        request_path: request.path,
        request_digest: request_digest
      }
    end

    def response_attributes(response)
      {
        response_status: response.status,
        response_body: response_body(response),
        response_headers: replayable_response_headers(response.headers)
      }
    end

    def replayable_response_headers(headers)
      REPLAYABLE_RESPONSE_HEADERS.index_with { |name| headers[name] }.compact
    end

    def key
      request.headers['Idempotency-Key'].to_s.presence
    end

    def mutating_request?
      request.post? || request.patch? || request.put? || request.delete?
    end

    def same_request?(record)
      record.account_id == credential.account_id &&
        record.request_method == request.request_method &&
        record.request_path == request.path &&
        record.request_digest == request_digest
    end

    def request_digest
      @request_digest ||= Digest::SHA256.hexdigest(
        JSON.generate(
          method: request.request_method,
          path: request.path,
          params: request.filtered_parameters.except('controller', 'action')
        )
      )
    end

    def response_body(response)
      JSON.parse(response.body.presence || '{}')
    rescue JSON::ParserError
      {}
    end
  end
end
