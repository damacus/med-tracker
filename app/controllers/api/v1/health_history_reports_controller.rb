module Api
  module V1
    class HealthHistoryReportsController < BaseController
      include ReportRendering

      def show
        authorize :report, :index?
        authorize selected_person, :download_health_history?
        result = Reports::GpHealthHistoryQuery.new(person: selected_person, **date_range.to_h,
                                                   include_medication_takes: include_medication_takes?).call
        deliver_report(data: HealthHistoryReportSerializer.new(result, **report_context).as_json,
                       pdf: Reports::HealthHistoryPdf.new(result: result, **report_context,
                                                          include_medication_takes: include_medication_takes?),
                       filename: "medtracker-health-history-#{date_range.start_date}-to-#{date_range.end_date}.pdf",
                       event_type: 'health_history_report.downloaded', metadata: download_metadata)
      end

      private

      def selected_person
        @selected_person ||= begin
          scope = PersonPolicy::Scope.new(pundit_user, policy_scope(Person)).resolve_for(:manage)
          find_api_record(scope, params.expect(:person_id))
        end
      end

      def date_range
        @date_range ||= begin
          %i[start_date end_date].each do |field|
            value = params[field]
            raise ArgumentError if value.present? && !value.to_s.match?(/\A\d{4}-\d{2}-\d{2}\z/)
          end
          Reports::GpDateRange.parse(start_date: params[:start_date], end_date: params[:end_date])
        end
      end

      def report_context
        date_range.to_h.merge(generated_at: generated_at)
      end

      def include_medication_takes?
        value = params[:include_medication_takes]
        raise ArgumentError if value.present? && %w[0 1].exclude?(value.to_s)

        value == '1'
      end

      def download_metadata
        { person_id: selected_person.id, start_date: date_range.start_date.iso8601,
          end_date: date_range.end_date.iso8601, include_medication_takes: include_medication_takes? }
      end
    end
  end
end
