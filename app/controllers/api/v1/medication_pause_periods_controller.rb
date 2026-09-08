module Api
  module V1
    class MedicationPausePeriodsController < BaseController
      SOURCES = { 'schedule' => Schedule, 'person_medication' => PersonMedication }.freeze
      PRELOADS = [
        :schedule, :person_medication,
        { recorded_by_membership: :person, resumed_by_membership: :person }
      ].freeze

      rescue_from ActiveRecord::RecordInvalid do |exception|
        render_validation_errors(exception.record)
      end

      def index
        scope = visible_periods
        if params[:source_type].present? || params[:source_id].present?
          source = find_source(params, include_retired: true)
          authorize source, :show?
          scope = scope.where(source_foreign_key(source) => source.id)
        end
        render_collection(scope, serializer: MedicationPausePeriodSerializer, includes: PRELOADS)
      end

      def create
        attributes = params.expect(medication_pause_period: %i[source_type source_id reason note])
        source = find_source(attributes)
        authorize source, :update?
        return render_unprocessable('A supported pause reason is required') unless valid_context?(attributes)

        period = MedicationAdministration::PausePeriodService.new(
          source:, membership: current_membership, reason: attributes[:reason], note: attributes[:note],
          started_at: Time.current
        ).call
        render_period(period, status: :created)
      end

      def resume
        period = visible_periods.find_by!(portable_id: params.expect(:id))
        source = period.schedule || period.person_medication
        raise ActiveRecord::RecordNotFound if source.retired_at.present?

        authorize source, :update?
        return unless fresh_api_record?(period)

        period = MedicationAdministration::ResumePeriodService.new(
          source:, membership: current_membership, ended_at: Time.current, period:
        ).call
        render_period(period)
      end

      private

      def authorize_api_replay!
        source = replay_source
        raise ActiveRecord::RecordNotFound if source.retired_at.present?

        authorize source, :update?
      end

      def replay_source
        if action_name == 'create'
          attributes = params.expect(medication_pause_period: %i[source_type source_id])
          find_source(attributes)
        else
          period = visible_periods.find_by!(portable_id: params.expect(:id))
          period.schedule || period.person_medication
        end
      end

      def apply_collection_filters(scope)
        super.reorder(created_at: :desc, id: :desc)
      end

      def visible_periods
        visibility.periods
      end

      def visibility
        @visibility ||= Api::MedicationPausePeriodVisibility.new(
          household: current_household, person_scope: policy_scope(Person)
        )
      end

      def find_source(attributes, include_retired: false)
        klass = SOURCES[attributes[:source_type]]
        raise ActiveRecord::RecordNotFound unless klass

        identifier = attributes[:source_id]
        raise InvalidContractValue, 'source_id' unless identifier.is_a?(String)

        scope = include_retired ? visibility.sources(klass) : policy_scope(klass)
        scope.find_by!(portable_id: identifier)
      end

      def source_foreign_key(source)
        source.is_a?(Schedule) ? :schedule_id : :person_medication_id
      end

      def valid_context?(attributes)
        MedicationPausePeriod::PUBLIC_REASONS.include?(attributes[:reason]) &&
          (attributes[:note].nil? || attributes[:note].is_a?(String))
      end

      def render_period(period, status: :ok)
        period = MedicationPausePeriod.includes(*PRELOADS).find(period.id)
        render_resource(period, serializer: MedicationPausePeriodSerializer, status:)
      end
    end
  end
end
