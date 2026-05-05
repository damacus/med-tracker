# frozen_string_literal: true

module PersonMedicationTakeRenderable
  extend ActiveSupport::Concern

  private

  def take_person_medication(taken_at)
    result = TakeMedicationService.new.call(
      source: @person_medication,
      amount_override: params[:amount_ml],
      taken_from_medication_id: requested_taken_from_medication_id,
      user: current_user,
      taken_at: taken_at
    )
    log_person_medication_invalid_take_attempt if result.error == :invalid_amount
    result
  end

  def log_person_medication_invalid_take_attempt
    log_invalid_take_attempt(source: 'person_medication', amount: nil,
                             metadata: { person_medication_id: @person_medication.id,
                                         medication_id: @person_medication.medication_id })
  end

  def handle_person_medication_take_failure(result)
    handle_take_medication_failure(result.error, scope: 'person_medications')
  end

  def render_person_medication_take_success(take)
    @take = take
    respond_to do |format|
      format.html { redirect_back_or_to person_path(@person), notice: t('person_medications.medication_taken') }
      format.turbo_stream do
        flash.now[:notice] = t('person_medications.medication_taken')
        streams = build_timeline_streams_for(@person_medication.reload, @take)
        streams << turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice]))
        render turbo_stream: streams
      end
    end
  end
end
