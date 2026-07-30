# frozen_string_literal: true

module Observability
  class CorrelationContext
    DEFAULT_LIFETIME = 24.hours
    UUID_PATTERN = /\A[0-9a-f]{8}-(?:[0-9a-f]{4}-){3}[0-9a-f]{12}\z/

    attr_reader :workflow_id, :causation_id, :attempt_id, :expires_at

    def self.start(now: Time.current, lifetime: DEFAULT_LIFETIME, id_generator: -> { SecureRandom.uuid })
      new(
        workflow_id: id_generator.call,
        causation_id: nil,
        attempt_id: nil,
        expires_at: now + lifetime,
        id_generator:
      )
    end

    def self.from_propagation(payload, now: Time.current, lifetime: DEFAULT_LIFETIME,
                              rotate_expired: true, id_generator: -> { SecureRandom.uuid })
      workflow_id = valid_identifier(payload['workflow.id'])
      expires_at = parse_expiry(payload['expires_at'])
      expired = expires_at.nil? || expires_at <= now
      return start(now:, lifetime:, id_generator:) if workflow_id.nil? || (expired && rotate_expired)
      return if expired

      new(
        workflow_id:,
        causation_id: valid_identifier(payload['causation.id']),
        attempt_id: valid_identifier(payload['attempt.id']),
        expires_at:,
        id_generator:
      )
    end

    def self.valid_identifier(value)
      value if value.to_s.match?(UUID_PATTERN)
    end
    private_class_method :valid_identifier

    def self.parse_expiry(value)
      Time.iso8601(value.to_s)
    rescue ArgumentError
      nil
    end
    private_class_method :parse_expiry

    def initialize(workflow_id:, causation_id:, attempt_id:, expires_at:, id_generator:)
      raise ArgumentError, 'invalid workflow identifier' unless workflow_id.to_s.match?(UUID_PATTERN)

      @workflow_id = workflow_id
      @causation_id = causation_id
      @attempt_id = attempt_id
      @expires_at = expires_at
      @id_generator = id_generator
      freeze
    end

    def caused_by(event_id, preserve_attempt: false)
      self.class.new(
        workflow_id:,
        causation_id: self.class.send(:valid_identifier, event_id),
        attempt_id: preserve_attempt ? attempt_id : nil,
        expires_at:,
        id_generator:
      )
    end

    def next_attempt
      self.class.new(
        workflow_id:,
        causation_id:,
        attempt_id: id_generator.call,
        expires_at:,
        id_generator:
      )
    end

    def to_propagation
      {
        'workflow.id' => workflow_id,
        'causation.id' => causation_id,
        'attempt.id' => attempt_id,
        'expires_at' => expires_at.utc.iso8601(3)
      }
    end

    def to_event_fields
      {
        'medtracker.workflow.id' => workflow_id,
        'medtracker.causation.id' => causation_id,
        'medtracker.attempt.id' => attempt_id
      }.compact
    end

    private

    attr_reader :id_generator
  end
end
