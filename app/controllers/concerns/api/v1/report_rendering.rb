module Api
  module V1
    module ReportRendering
      extend ActiveSupport::Concern

      included do
        before_action { response.set_header('Cache-Control', 'no-store') }
        rescue_from Reports::PdfRenderer::Error, ActiveRecord::RecordInvalid, with: :render_report_failure
        rescue_from ArgumentError, with: :render_invalid_report
      end

      private

      def deliver_report(data:, pdf:, filename:, event_type:, metadata:)
        format = report_format
        content = format == 'pdf' ? pdf.render : { data: data }
        Audit::Event.record!(household: current_household, request: request, event_type: event_type,
                             metadata: metadata.merge(format: format, outcome: 'success'))
        if format == 'pdf'
          send_data content, filename: filename, type: 'application/pdf', disposition: 'attachment'
        else
          render json: content
        end
      end

      def report_format
        format = params[:format].presence&.to_s || 'json'
        raise ArgumentError unless %w[json pdf].include?(format)

        format
      end

      def generated_at
        @generated_at ||= Time.current
      end

      def render_invalid_report
        render_unprocessable('Report parameters are invalid')
      end

      def render_report_failure(error)
        Observability::DiagnosticEvent.emit(component: :api_report_export, reason: :operation_failed,
                                            severity: :error, error: error)
        render_api_error(code: 'report_unavailable', message: 'Report is temporarily unavailable',
                         status: :service_unavailable)
      end
    end
  end
end
