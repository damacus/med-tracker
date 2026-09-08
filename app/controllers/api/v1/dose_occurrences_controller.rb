module Api
  module V1
    class DoseOccurrencesController < BaseController
      def index
        source = find_api_record(policy_scope(Schedule), params.expect(:schedule_id))
        authorize source, :show?
        rows = MedicationAdministration::OccurrenceProjection.new(
          source: source, start_date: occurrence_date(:start_date), end_date: occurrence_date(:end_date)
        ).call
        render json: { data: rows.map { |row| DoseOccurrenceSerializer.new(row).as_json } }
      rescue ArgumentError
        render_unprocessable('A valid date range of at most 31 days is required')
      end

      private

      def occurrence_date(name)
        value = params[name].to_s
        raise ArgumentError unless value.match?(/\A\d{4}-\d{2}-\d{2}\z/)

        Date.iso8601(value)
      end
    end
  end
end
