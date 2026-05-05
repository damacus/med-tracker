# frozen_string_literal: true

module ScheduleTakeRenderable
  extend ActiveSupport::Concern

  private

  def take_schedule(taken_at)
    result = TakeMedicationService.new.call(
      source: @schedule,
      amount_override: params[:amount_ml],
      taken_from_medication_id: requested_taken_from_medication_id,
      user: current_user,
      taken_at: taken_at
    )
    log_schedule_invalid_take_attempt if result.error == :invalid_amount
    result
  end

  def log_schedule_invalid_take_attempt
    log_invalid_take_attempt(source: 'schedule', amount: nil,
                             metadata: { schedule_id: @schedule.id,
                                         medication_id: @schedule.medication_id })
  end

  def handle_schedule_take_failure(result)
    handle_take_medication_failure(result.error, scope: 'schedules')
  end

  def render_schedule_take_success(take)
    @take = take
    respond_to do |format|
      format.html { redirect_back_or_to person_path(@person), notice: t('schedules.medication_taken') }
      format.turbo_stream do
        flash.now[:notice] = t('schedules.medication_taken')
        streams = build_timeline_streams_for(@schedule.reload, @take)
        streams << turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice]))
        render turbo_stream: streams
      end
    end
  end
end
