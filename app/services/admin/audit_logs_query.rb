# frozen_string_literal: true

module Admin
  class AuditLogsQuery
    Result = Data.define(:versions, :total_count, :page, :per_page, :item_types, :events)
    DEFAULT_ITEM_TYPES = %w[User Account Person CarerRelationship Location LocationMembership
                            MedicationTake Medication MedicationDosageOption Schedule NotificationPreference
                            AppSettings ExternalMedicineLookup AiMedicationSuggestion AuthenticationToken].freeze
    DEFAULT_EVENTS = ['create', 'update', 'destroy', 'restock', 'adjust inventory',
                      'dose_decrement', 'mark_as_ordered', 'mark_as_received',
                      'ai_medication/suggestion',
                      'auth_token/api_session/created',
                      'auth_token/api_session/rotated',
                      'auth_token/api_session/revoked',
                      'auth_token/api_session/expired',
                      'auth_token/push_subscription/created',
                      'auth_token/push_subscription/revoked',
                      'auth_token/native_device_token/created',
                      'auth_token/native_device_token/revoked',
                      'auth_token/otp_key/created',
                      'auth_token/otp_key/revoked',
                      'auth_token/recovery_codes/created',
                      'auth_token/webauthn_credential/created',
                      'auth_token/webauthn_credential/revoked',
                      'auth_token/verification_key/created',
                      'auth_token/verification_key/revoked',
                      'auth_token/password_reset_key/created',
                      'auth_token/password_reset_key/revoked',
                      'auth_token/login_change_key/created',
                      'auth_token/login_change_key/revoked',
                      'auth_token/remember_key/created',
                      'auth_token/remember_key/revoked'].freeze

    attr_reader :scope, :filters, :page, :per_page

    def initialize(scope:, filters:, page:, per_page:)
      @scope = scope
      @filters = filters
      @page = [page.to_i, 1].max
      @per_page = per_page
    end

    def call
      Result.new(
        versions: filtered_scope.limit(per_page).offset((page - 1) * per_page),
        total_count: filtered_scope.count,
        page: page,
        per_page: per_page,
        item_types: filter_values(DEFAULT_ITEM_TYPES, :item_type),
        events: filter_values(DEFAULT_EVENTS, :event)
      )
    end

    private

    def filtered_scope
      relation = scope.order(created_at: :desc)
      relation = relation.where(item_type: item_type) if item_type.present?
      relation = relation.where('event LIKE ?', "#{ActiveRecord::Base.sanitize_sql_like(event)}%") if event.present?
      relation = relation.where(whodunnit: whodunnit) if whodunnit.present?
      relation
    end

    def item_type
      filters[:item_type].to_s.presence
    end

    def event
      filters[:event].to_s.presence
    end

    def whodunnit
      filters[:whodunnit].to_s.presence
    end

    def filter_values(defaults, column)
      (defaults + distinct_values(column)).uniq.sort
    end

    def distinct_values(column)
      scope.reorder(nil).where.not(column => [nil, '']).distinct.pluck(column)
    end
  end
end
