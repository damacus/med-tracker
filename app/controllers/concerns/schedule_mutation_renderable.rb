# frozen_string_literal: true

module ScheduleMutationRenderable
  extend ActiveSupport::Concern

  private

  def render_schedule_create_success
    respond_to do |format|
      format.html { redirect_to person_path(@person), notice: t('schedules.created') }
      format.turbo_stream do
        flash.now[:notice] = t('schedules.created')
        render turbo_stream: [
          turbo_stream.update('modal', ''),
          turbo_stream.replace("person_#{@person.id}", Components::People::PersonCard.new(person: @person.reload)),
          turbo_stream.replace("person_show_#{@person.id}", person_show_view(@person.reload)),
          turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
        ]
      end
    end
  end

  def render_schedule_create_failure
    @medications = medication_options_query.call
    render_new_schedule_form(status: :unprocessable_content)
  end

  def render_schedule_update_success
    respond_to do |format|
      format.html { redirect_to person_path(@person), notice: t('schedules.updated') }
      format.turbo_stream do
        flash.now[:notice] = t('schedules.updated')
        render turbo_stream: [
          turbo_stream.update('modal', ''),
          turbo_stream.replace("person_show_#{@person.id}", person_show_view(@person.reload)),
          turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
        ]
      end
    end
  end

  def render_schedule_update_failure
    @medications = medication_options_query.call
    render_edit_schedule_form(status: :unprocessable_content)
  end

  def render_schedule_destroy_success
    respond_to do |format|
      format.html { redirect_back_or_to person_path(@person), notice: t('schedules.deleted') }
      format.turbo_stream do
        flash.now[:notice] = t('schedules.deleted')
        render turbo_stream: [
          turbo_stream.replace("person_show_#{@person.id}", person_show_view(@person.reload)),
          turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
        ]
      end
    end
  end
end
