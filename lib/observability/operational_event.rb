# frozen_string_literal: true

module Observability
  class OperationalEvent
    include OperationalEventSanitizer
    include OperationalEventValidator
    include OperationalEventPayloadBuilder

    SCHEMA_VERSION = 1
    DATASET = 'medtracker.application'
    EVENT_NAMES = {
      medication_take_attempted: 'medication_take.attempted',
      medication_take_persisted: 'medication_take.persisted',
      medication_take_committed: 'medication_take.committed',
      medication_take_rolled_back: 'medication_take.rolled_back',
      medication_take_blocked: 'medication_take.blocked',
      medication_take_failed: 'medication_take.failed',
      low_stock_threshold_reached: 'low_stock.threshold_reached',
      audit_delivery_backlog_evaluated: 'audit_delivery.backlog_evaluated',
      request_throttled: 'request.throttled',
      domain_event_subscriber_failed: 'domain_event.subscriber_failed',
      notification_stage: 'notification.stage',
      diagnostic: 'diagnostic.occurred',
      observability_canary: 'observability.canary',
      http_request_completed: 'http.request.completed',
      job_completed: 'job.completed'
    }.freeze
    SEVERITIES = %i[debug info warn error fatal].freeze
    OUTCOMES = %i[success failure unknown].freeze
    REASONS = %i[
      requested provisional recorded rules_blocked out_of_stock cooldown paused invalid_amount
      selection_required invalid_source create_failed persistence_failed threshold_reached
      backlog_healthy backlog_warning backlog_critical throttled subscriber_failed completed failed
      committed rolled_back household_unavailable unexpected_failure
      enqueued eligible past_occurrence suppressed duplicate retrying job_failed no_recipients
      no_due_dose no_active_push_subscriptions person_unavailable intent_recorded attempted
      provider_accepted partial_failure permanent_failure delivery_unknown preference_disabled
      no_medications invalid_schedule
      operation_failed configuration_warning configured throttled invalid_payload recovered
      canary_emitted
    ].freeze
    ATTRIBUTE_FIELDS = {
      source_category: 'medtracker.source.category',
      actor_role: 'medtracker.actor.role',
      workflow_stage: 'medtracker.workflow.stage',
      throttle_category: 'medtracker.throttle.category',
      backlog_severity: 'medtracker.backlog.severity',
      pending_count: 'medtracker.backlog.pending_count',
      oldest_age_seconds: 'medtracker.backlog.oldest_age_seconds',
      http_method: 'http.request.method',
      route: 'http.route',
      status_code: 'http.response.status_code',
      duration: 'event.duration',
      job_class: 'medtracker.job.class',
      job_queue: 'medtracker.job.queue',
      notification_kind: 'medtracker.notification.kind',
      notification_channel: 'medtracker.notification.channel',
      notification_provider: 'medtracker.notification.provider',
      recipient_count: 'medtracker.notification.recipient_count',
      canary_kind: 'medtracker.canary.kind',
      diagnostic_component: 'medtracker.diagnostic.component'
    }.freeze
    EVENT_ATTRIBUTE_KEYS = {
      medication_take_attempted: %i[source_category actor_role],
      medication_take_persisted: %i[source_category actor_role],
      medication_take_committed: %i[source_category actor_role],
      medication_take_rolled_back: %i[source_category actor_role],
      medication_take_blocked: %i[source_category actor_role],
      medication_take_failed: %i[source_category actor_role],
      low_stock_threshold_reached: %i[source_category],
      audit_delivery_backlog_evaluated: %i[backlog_severity pending_count oldest_age_seconds],
      request_throttled: %i[throttle_category],
      domain_event_subscriber_failed: %i[workflow_stage],
      notification_stage: %i[
        notification_kind workflow_stage notification_channel notification_provider recipient_count
      ],
      diagnostic: %i[diagnostic_component],
      observability_canary: %i[canary_kind],
      http_request_completed: %i[http_method route status_code duration],
      job_completed: %i[job_class job_queue duration]
    }.freeze
    CATEGORICAL_VALUES = {
      source_category: %w[schedule person_medication unknown],
      actor_role: %w[admin clinician self carer parent unknown],
      backlog_severity: %w[healthy warning critical unknown],
      throttle_category: %w[medication_lookup ai_medication_suggestions unknown],
      workflow_stage: %w[
        take_attempted take_recorded take_blocked_by_rules take_errors dose_taken
        low_stock_threshold_reached audit_delivery_backlog rack_attack_throttled
        reminder_enqueue reminder_eligibility reminder_delivery missed_dose_evaluation
        notification_intent recipient_attempt channel_attempt provider_outcome
        low_stock_evaluation low_stock_delivery job_execution
      ],
      notification_kind: %w[dose_due missed_dose low_stock test unknown],
      notification_channel: %w[web_push native_push mixed none unknown],
      notification_provider: %w[web_push apns fcm unknown],
      canary_kind: %w[application_event job],
      diagnostic_component: %w[
        audit_log medication_guard test_push nhs_dmd_import mcp_audit passkey_security
        opentelemetry ai_audit ai_suggestion auth_token_audit barcode_catalog external_lookup
        medication_finder nhs_dmd_search nhs_dmd_supplementary nhs_dmd_vmp nhs_website_content
        open_food_facts open_products_facts passkey_renderer webauthn_renderer oidc rate_limit
        mailpit database_pool span_sanitizer observability_canary
      ]
    }.freeze
    MESSAGES = {
      medication_take_attempted: 'Medication administration attempted',
      observability_emission_failed: 'Operational event emission failed'
    }.freeze

    attr_reader :severity

    def self.build(**)
      new(**)
    end

    DATASETS = %w[medtracker.application medtracker.request medtracker.job].freeze

    def initialize(**provided_options)
      options = event_options(provided_options)
      options[:service] ||= self.class.send(:default_service)
      validate!(options)
      @severity = options.fetch(:severity).to_sym
      @build_options = options.except(:event_id)
      @payload = build_payload(options).freeze
      freeze
    end

    def to_h
      payload
    end

    def to_json(*)
      payload.to_json
    end

    def reemit
      self.class.build(**build_options)
    end

    class << self
      private

      def default_service
        {
          name: 'medtracker',
          version: ENV['APP_VERSION'].presence || MedTracker::VERSION,
          environment: Rails.env.to_s
        }
      end
    end

    private

    attr_reader :payload, :build_options
  end
end
