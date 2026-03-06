# frozen_string_literal: true

module ScheduleModalRenderable
  private

  def render_schedule_form(schedule:, medications:, title:, **options)
    editing = options.fetch(:editing, false)
    back_path = options[:back_path]
    status = options.fetch(:status, :ok)
    is_modal = request.headers['Turbo-Frame'] == 'modal'

    respond_to do |format|
      format.html do
        if is_modal
          render schedule_modal_component(
            schedule: schedule,
            medications: medications,
            title: title,
            editing: editing,
            back_path: back_path
          ), layout: false, status: status
        else
          render schedule_form_view(
            schedule: schedule,
            medications: medications,
            editing: editing
          ), status: status
        end
      end
      format.turbo_stream do
        render turbo_stream: turbo_stream.replace(
          'modal',
          schedule_modal_component(
            schedule: schedule,
            medications: medications,
            title: title,
            editing: editing,
            back_path: back_path
          )
        ), status: status
      end
    end
  end

  def schedule_modal_component(schedule:, medications:, title:, editing: false, back_path: nil)
    Components::Schedules::Modal.new(
      schedule: schedule,
      person: @person,
      medications: medications,
      title: title,
      editing: editing,
      back_path: back_path
    )
  end

  def schedule_form_view(schedule:, medications:, editing: false)
    if editing
      Components::Schedules::EditView.new(schedule: schedule, person: @person, medications: medications)
    else
      Components::Schedules::NewView.new(schedule: schedule, person: @person, medications: medications)
    end
  end
end
