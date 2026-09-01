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
  rescue Reports::PdfRenderer::Error => error
    Rails.logger.error("PDF rendering failed report_type=medication_review exception_class=#{error.class.name}")
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
    scope = policy_scope(MedicationReviewPrompt).includes(:person)
    scope = filter_person(scope)
    scope = filter_status(scope)
    scope.order(:person_id, :created_at, :id).to_a
  end

  def filter_person(scope)
    person_id = params.permit(:person_id).fetch(:person_id, nil)
    return scope if person_id.blank?
    return scope.none unless person_id.match?(/\A\d+\z/)

    scope.where(person_id: person_id)
  end

  def filter_status(scope)
    status = params.permit(:status).fetch(:status, nil)
    return scope.visible_by_default if status.blank?
    return scope.none unless status.in?(MedicationReviewPrompt::STATUSES)

    scope.where(status: status)
  end
end
