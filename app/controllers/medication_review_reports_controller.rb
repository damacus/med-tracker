# frozen_string_literal: true

class MedicationReviewReportsController < ApplicationController
  def show
    authorize MedicationReviewPrompt, :index?
    sync_review_prompts
    rendered_pdf = pdf.render
    response.headers['Cache-Control'] = 'no-store'

    send_data rendered_pdf,
              filename: "medtracker-medication-review-#{Date.current.iso8601}.pdf",
              type: 'application/pdf',
              disposition: 'attachment'
  rescue Reports::PdfRenderer::Error => e
    Observability::DiagnosticEvent.emit(
      component: :medication_review_pdf_export,
      reason: :operation_failed,
      severity: :error,
      error: e
    )
    redirect_to medication_review_prompts_path, alert: t('reports.export.pdf_unavailable')
  end

  private

  def sync_review_prompts
    MedicationReviewPromptSync.new(people: policy_scope(Person)).call
  end

  def pdf
    Reports::MedicationReviewPdf.new(prompts: filtered_prompts, generated_at: Time.current)
  end

  def filtered_prompts
    scope = filter_person(policy_scope(MedicationReviewPrompt))
    MedicationReviewReportQuery.new(scope: scope, status: params[:status]).call.to_a
  rescue ArgumentError
    []
  end

  def filter_person(scope)
    person_id = params.permit(:person_id).fetch(:person_id, nil)
    return scope if person_id.blank?
    return scope.none unless person_id.match?(/\A\d+\z/)

    scope.where(person_id: person_id)
  end
end
