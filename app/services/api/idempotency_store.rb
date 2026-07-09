# frozen_string_literal: true

module Api
  class IdempotencyStore
    EXPIRY = 24.hours

    Result = Data.define(:record, :replayed, :conflict)

    def initialize(request:, credential:, household:)
      @request = request
      @credential = credential
      @household = household
    end

    def active?
      key.present? && mutating_request? && household.present? && credential.present?
    end

    def lookup
      record = ApiIdempotencyKey.find_by(household: household, key: key)
      return Result.new(record: nil, replayed: false, conflict: false) unless record

      Result.new(record: record, replayed: same_request?(record), conflict: !same_request?(record))
    end

    def store!(response)
      return unless active? && response.status < 500

      ApiIdempotencyKey.create!(idempotency_attributes(response))
    rescue ActiveRecord::RecordInvalid, ActiveRecord::RecordNotUnique
      nil
    end

    private

    attr_reader :request, :credential, :household

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
        response_body: response_body(response)
      }
    end

    def key
      request.headers['Idempotency-Key'].to_s.presence
    end

    def mutating_request?
      request.post? || request.patch? || request.put? || request.delete?
    end

    def same_request?(record)
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
