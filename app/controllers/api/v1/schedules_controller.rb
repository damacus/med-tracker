# frozen_string_literal: true

module Api
  module V1
    class SchedulesController < BaseController
      def index
        authorize Schedule
        render_collection(
          policy_scope(Schedule), serializer: ScheduleSerializer, includes: source_preloads,
                                  serializer_options: method(:source_serializer_options)
        )
      end

      def show
        schedule = find_api_record(policy_scope(Schedule).includes(*source_preloads), params.expect(:id))
        authorize schedule

        render_source(schedule)
      end

      def create
        attributes = schedule_params
        person = policy_scope(Person).find(attributes.delete(:person_id))
        authorize person, :show?
        assert_medication_access!(attributes[:medication_id])
        schedule = person.schedules.build(attributes)
        authorize schedule

        return render_validation_errors(schedule) unless schedule.save

        render_source(schedule.reload, status: :created)
      end

      def update
        schedule = find_api_record(policy_scope(Schedule).includes(*source_preloads), params.expect(:id))
        authorize schedule
        return unless fresh_api_record?(schedule)

        attributes = schedule_update_params
        assert_medication_access!(attributes[:medication_id]) if attributes[:medication_id].present?

        return render_validation_errors(schedule) unless schedule.update(attributes)

        render_source(schedule.reload)
      end

      def pause
        update_pause_state(:pause!)
      end

      def resume
        update_pause_state(:resume!)
      end

      private

      def authorize_api_replay!
        return super unless %w[pause resume].include?(action_name)

        source = find_api_record(policy_scope(Schedule), params.expect(:id))
        authorize source, :update?
      end

      def source_preloads
        [:person, :medication, { medication_pause_periods: MedicationPausePeriodsController::PRELOADS }]
      end

      def update_pause_state(method_name)
        schedule = find_api_record(policy_scope(Schedule).includes(*source_preloads), params.expect(:id))
        authorize schedule, :update?
        schedule.public_send(method_name)
        schedule.association(:medication_pause_periods).reset
        ActiveRecord::Associations::Preloader.new(records: [schedule], associations: source_preloads).call
        render_source(schedule)
      end

      def render_source(schedule, status: :ok)
        render_resource(schedule, serializer: ScheduleSerializer, status:,
                                  serializer_options: method(:source_serializer_options))
      end

      def source_serializer_options(schedule)
        { can_manage: policy(schedule).update? }
      end

      def schedule_params
        reject_numeric_contract_values!(%w[person_id medication_id source_dosage_option_id dose_amount min_hours_between_doses amount])
        attributes = params.expect(
          schedule: [
            :person_id,
            :medication_id,
            :dose_amount,
            :dose_unit,
            :source_dosage_option_id,
            :frequency,
            :start_date,
            :end_date,
            :notes,
            :max_daily_doses,
            :min_hours_between_doses,
            :dose_cycle,
            :schedule_type,
            :schedule_config,
            { schedule_config: {} }
          ]
        )
        if attributes[:person_id].present?
          attributes[:person_id] = api_record_id(policy_scope(Person), attributes[:person_id])
        end
        if attributes[:medication_id].present?
          attributes[:medication_id] = api_record_id(policy_scope(Medication), attributes[:medication_id])
        end
        if attributes[:source_dosage_option_id].present?
          attributes[:source_dosage_option_id] = api_record_id(
            policy_scope(MedicationDosageOption),
            attributes[:source_dosage_option_id]
          )
        end
        attributes
      end

      def schedule_update_params
        schedule_params.except(:person_id)
      end

      def assert_medication_access!(medication_id)
        policy_scope(Medication).find(medication_id)
      end
    end
  end
end
