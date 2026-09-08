class DoseOccurrencesController < ApplicationController
  before_action :set_source
  before_action :set_outcome, only: %i[edit update]

  def new
    render_form
  end

  def edit
    render_correction
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

  def update
    attributes = params.expect(dose_occurrence: %i[resolution etag taken_at taken_from_medication_id client_uuid])
    correct_outcome(attributes)
    redirect_to dashboard_path(dashboard_person_id: @source.person_id),
                notice: t('dose_outcomes.corrected'), status: :see_other
  rescue MedicationAdministration::OccurrenceResolver::Error, ActiveRecord::RecordInvalid, ArgumentError
    render_correction(error: t('dose_outcomes.invalid'), status: :unprocessable_content)
  end

  private

  def set_outcome
    @outcome = @source.medication_dose_occurrences.find(params.expect(:id))
  end

  def correct_outcome(attributes)
    resolver = MedicationAdministration::OccurrenceResolver.new(source: @source, authorization: pundit_user)
    case attributes[:resolution]
    when 'reopen'
      reopen_outcome(resolver, attributes)
    when 'take'
      take_outcome(resolver, attributes)
    else
      raise ArgumentError
    end
  end

  def reopen_outcome(resolver, attributes)
    authorize @source, :update?
    resolver.call(key: outcome_key, action: 'reopen', if_match: attributes[:etag])
  end

  def take_outcome(resolver, attributes)
    resolver.take(key: outcome_key, taken_at: correction_time(attributes[:taken_at]), if_match: attributes[:etag],
                  **attributes.slice(:taken_from_medication_id, :client_uuid).to_h.symbolize_keys)
  end

  def correction_time(value)
    time = Time.zone.iso8601(value)
    raise ArgumentError if time > Time.current

    time
  end

  def outcome_key
    MedicationAdministration::OccurrenceProjection.new(
      source: @source, start_date: @outcome.window_starts_on, end_date: @outcome.window_starts_on
    ).call.find { |row| row.position == @outcome.position }.key
  end

  def render_correction(error: nil, status: :ok)
    @outcome.reload
    medications = MedicationStockSourceResolver.new(user: pundit_user, source: @source).available_medications
    render Components::DoseOccurrences::EditView.new(
      source: @source, outcome: @outcome, medications: medications,
      can_reopen: policy(@source).update?, error: error
    ), status: status
  end

  def set_source
    @source = policy_scope(Schedule).includes(:person, :medication).find(params.expect(:schedule_id))
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
