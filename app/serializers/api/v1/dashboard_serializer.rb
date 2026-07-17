# frozen_string_literal: true

module Api
  module V1
    class DashboardSerializer
      BLOCKING_STATUSES = %i[cooldown max_reached paused out_of_stock selection_required].freeze
      Context = Data.define(:visible_people, :selected_person, :schedule_query, :date, :server_time, :time_zone)

      def initialize(context)
        @context = context
      end

      def as_json(*)
        dashboard_metadata.merge(serialized_dashboard_tasks)
      end

      private

      attr_reader :context

      delegate :visible_people, :selected_person, :schedule_query, :date, :server_time, :time_zone, to: :context

      def dashboard_metadata
        {
          format: 'medtracker.dashboard.v1',
          server_time: server_time.iso8601,
          time_zone: time_zone,
          date: date.iso8601,
          people: visible_people.map { |person| person_summary(person) },
          selected_person: selected_person && person_summary(selected_person)
        }
      end

      def serialized_dashboard_tasks
        routine_tasks = serialize_tasks(schedule_query.routine_tasks, routine: true)
        as_needed_tasks = serialize_as_needed_tasks
        completed_takes = serialize_completed_takes

        {
          summary_counts: summary_counts(routine_tasks, as_needed_tasks, completed_takes),
          next_task: next_task(routine_tasks, as_needed_tasks),
          routine_tasks: routine_tasks,
          as_needed_tasks: as_needed_tasks,
          recent_completed_takes: completed_takes
        }
      end

      def serialize_as_needed_tasks
        rows = schedule_query.as_needed_by_person.values.flatten
        serialize_tasks(rows, routine: false)
      end

      def serialize_tasks(rows, routine:)
        rows.map { |row| task_data(row, routine:) }
      end

      def task_data(row, routine:)
        source = row.fetch(:source)
        status = task_status(row, routine:)

        task_identity(row, source).merge(task_state(row, status))
      end

      def task_identity(row, source)
        {
          source_type: source_type(source),
          source_id: source.portable_id,
          person: person_summary(row.fetch(:person)),
          medication: medication_summary(source.medication),
          dose: effective_dose(source),
          scheduled_at: row[:scheduled_at]&.iso8601
        }
      end

      def task_state(row, status)
        {
          status: status.to_s,
          blocking_reason: BLOCKING_STATUSES.include?(status) ? status.to_s : nil,
          daily_progress: daily_progress(row),
          can_record: row.fetch(:can_record, BLOCKING_STATUSES.exclude?(status)),
          stock_source_choices: serialize_stock_source_choices(row[:stock_source_choices])
        }
      end

      def daily_progress(row)
        { count: row.fetch(:daily_dose_count), limit: row[:daily_dose_limit] }
      end

      def serialize_stock_source_choices(choices)
        Array(choices).map do |medication|
          {
            id: medication.portable_id,
            medication: medication_summary(medication),
            location: {
              id: medication.location.portable_id,
              name: medication.location.name
            },
            current_supply: medication.current_supply&.to_f
          }
        end
      end

      def task_status(row, routine:)
        status = row.fetch(:status).to_sym
        return status unless status == :upcoming && routine
        return :due if row[:scheduled_at].blank? || row[:scheduled_at] <= schedule_query.now

        :upcoming
      end

      def effective_dose(source)
        amount = if source.respond_to?(:effective_dose_amount)
                   source.effective_dose_amount(date)
                 else
                   source.dose_amount
                 end
        unit = source.respond_to?(:effective_dose_unit) ? source.effective_dose_unit(date) : source.dose_unit

        { amount: amount&.to_f, unit: unit }
      end

      def source_type(source)
        source.is_a?(Schedule) ? 'schedule' : 'person_medication'
      end

      def person_summary(person)
        { id: person.portable_id, name: person.name }
      end

      def medication_summary(medication)
        { id: medication.portable_id, name: medication.name }
      end

      def serialize_completed_takes
        schedule_query.today_takes_by_person.values.flatten
                      .uniq(&:id)
                      .sort_by { |take| [take.taken_at, take.id] }
                      .reverse
                      .first(20)
                      .map { |take| MedicationTakeSerializer.new(take).as_json }
      end

      def summary_counts(routine_tasks, as_needed_tasks, completed_takes)
        statuses = (routine_tasks + as_needed_tasks).pluck(:status).tally

        {
          routine: routine_tasks.size,
          as_needed: as_needed_tasks.size,
          completed: completed_takes.size,
          due: statuses.fetch('due', 0),
          upcoming: statuses.fetch('upcoming', 0),
          available: statuses.fetch('available', 0),
          blocked: statuses.slice(*BLOCKING_STATUSES.map(&:to_s)).values.sum
        }
      end

      def next_task(routine_tasks, as_needed_tasks)
        routine_tasks.find { |task| task[:status] == 'due' } ||
          routine_tasks.find { |task| task[:status] == 'upcoming' } ||
          as_needed_tasks.find { |task| task[:status] == 'available' }
      end
    end
  end
end
