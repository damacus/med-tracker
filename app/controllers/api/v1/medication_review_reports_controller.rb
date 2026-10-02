module Api
  module V1
    class MedicationReviewReportsController < BaseController
      include ReportRendering

      def show
        authorize MedicationReviewPrompt, :index?
        authorize selected_person, :show?
        raise ArgumentError if params.key?(:start_date) || params.key?(:end_date)

        prompts = report_prompts
        deliver_report(data: report_data(prompts),
                       pdf: Reports::MedicationReviewPdf.new(prompts: prompts, generated_at: generated_at),
                       filename: "medtracker-medication-review-#{generated_at.to_date.iso8601}.pdf",
                       event_type: 'medication_review_report.downloaded',
                       metadata: { person_id: selected_person.id, status: params[:status] })
      end

      private

      def selected_person
        @selected_person ||= find_api_record(policy_scope(Person), params.expect(:person_id))
      end

      def report_prompts
        scope = policy_scope(MedicationReviewPrompt).where(person: selected_person)
        filtered = MedicationReviewReportQuery.new(scope: scope, status: params[:status]).call
        MedicationReviewPromptSync.new(people: policy_scope(Person).where(id: selected_person.id)).call
        filtered.to_a
      end

      def report_data(prompts)
        { person: { id: selected_person.id.to_s, name: selected_person.name }, generated_at: generated_at.iso8601,
          prompts: prompts.map { |prompt| MedicationReviewPromptSerializer.new(prompt).as_json } }
      end
    end
  end
end
