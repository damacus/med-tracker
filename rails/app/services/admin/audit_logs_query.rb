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

  class AuditLogDetailQuery
    Result = Data.define(
      :actor_name,
      :medication_take,
      :next_version,
      :current_record,
      :source_record,
      :stock_source,
      :stock_location
    )

    attr_reader :version

    def initialize(version:)
      @version = version
    end

    def call
      medication_take = medication_take_record
      object_data = parsed_object_data
      next_version = following_version

      Result.new(
        actor_name: AuditActorResolver.new.name_for(version.whodunnit),
        medication_take: medication_take,
        next_version: next_version,
        current_record: current_record(next_version),
        source_record: source_record(medication_take, object_data),
        stock_source: stock_source(medication_take, object_data),
        stock_location: stock_location(medication_take, object_data)
      )
    end

    private

    def medication_take_record
      return unless version.item_type == 'MedicationTake' && version.item_id.present?

      MedicationTake.includes(
        :taken_from_location,
        taken_from_medication: :location,
        schedule: [{ medication: :location }, :person],
        person_medication: [{ medication: :location }, :person]
      ).find_by(id: version.item_id)
    end

    def following_version
      return if version.event == 'destroy'

      PaperTrail::Version.where(item_type: version.item_type, item_id: version.item_id)
                         .where('id > ?', version.id)
                         .order(:id)
                         .first
    end

    def current_record(next_version)
      return if next_version&.object.present?

      version.item_type.safe_constantize&.find_by(id: version.item_id)
    rescue StandardError
      nil
    end

    def source_record(medication_take, data)
      return medication_take.source if medication_take&.source

      schedule_id = data['schedule_id'].presence
      return Schedule.includes(:medication, :person).find_by(id: schedule_id) if schedule_id

      person_medication_id = data['person_medication_id'].presence
      return unless person_medication_id

      PersonMedication.includes(:medication, :person).find_by(id: person_medication_id)
    end

    def stock_source(medication_take, data)
      inventory_medication = medication_take&.inventory_medication
      return inventory_medication if inventory_medication

      medication_id = data['taken_from_medication_id'].presence
      Medication.find_by(id: medication_id) if medication_id
    end

    def stock_location(medication_take, data)
      inventory_location = medication_take&.inventory_location
      return inventory_location if inventory_location

      location_id = data['taken_from_location_id'].presence
      Location.find_by(id: location_id) if location_id
    end

    def parsed_object_data
      return {} unless version.item_type == 'MedicationTake' && version.object.present?

      data = YAML.safe_load(version.object, permitted_classes: ActiveRecord.yaml_column_permitted_classes)
      data.is_a?(Hash) ? data.stringify_keys : {}
    rescue StandardError
      {}
    end
  end
end
