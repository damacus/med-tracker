class DoseOccurrencesController < ApplicationController
  before_action :set_source

  def new
    render_form
  end

  def create
    attributes = params.expect(dose_occurrence: %i[key reason note])
    MedicationAdministration::OccurrenceResolver.new(source: @source, authorization: pundit_user).call(
      key: attributes[:key], action: 'not_taken', reason: attributes[:reason].presence, note: attributes[:note].presence
    )
    redirect_to dashboard_path(dashboard_person_id: @source.person_id),
                notice: t('dose_outcomes.saved'), status: :see_other
  rescue MedicationAdministration::OccurrenceResolver::Error, ActiveRecord::RecordInvalid
    render_form(error: t('dose_outcomes.invalid'), status: :unprocessable_content)
  end

  private

  def set_source
    @source = policy_scope(Schedule).find(params.expect(:schedule_id))
    authorize @source, :take_medication?
  end

  def render_form(error: nil, status: :ok)
    occurrences = MedicationAdministration::OccurrenceProjection.new(
      source: @source, start_date: Date.current, end_date: Date.current
    ).call.select { |row| row.outcome == 'open' && row.expected? && row.due? }
    values = params.fetch(:dose_occurrence, {}).slice(:reason, :note)
    render Components::DoseOccurrences::NewView.new(source: @source, occurrences: occurrences,
                                                    values: values, error: error), status: status
  end
end
