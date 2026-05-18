# frozen_string_literal: true

module Components
  module Schedules
    class Card
      class ActionsComponent < Components::Base
        attr_reader :schedule, :person, :presenter, :current_user

        def initialize(schedule:, person:, presenter:, current_user:)
          @schedule = schedule
          @person = person
          @presenter = presenter
          @current_user = current_user
          super()
        end

        def view_template
          CardFooter(class: 'px-8 pb-8 pt-2') do
            div(class: 'w-full @container', data: { testid: 'schedule-action-shell' }) do
              div(class: action_dock_classes, data: { testid: 'schedule-action-dock' }) do
                div(class: log_action_classes, data: { testid: 'schedule-log-action' }) do
                  render_past_dose_button
                end
                render_admin_actions if administrator?
              end
            end
          end
        end

        private

        def action_dock_classes
          'grid w-full grid-cols-[minmax(0,1fr)_3rem] items-center gap-x-4 gap-y-4 ' \
            'bg-surface-container-high p-4'
        end

        def log_action_classes
          if administrator?
            'order-1 col-span-2 min-w-0'
          else
            'col-span-2'
          end
        end

        def edit_action_classes
          'order-2 min-w-0'
        end

        def delete_action_classes
          'order-3 flex justify-center'
        end

        def render_past_dose_button
          render Components::Medications::PriorDayTakeAction.new(
            source: schedule,
            context: { person: person, current_user: current_user },
            amount: schedule.dose_amount,
            testid: "log-past-dose-schedule-#{schedule.id}",
            button: {
              variant: :filled,
              size: :lg,
              class: 'h-14 w-full rounded-full px-5 text-lg font-bold shadow-elevation-2'
            }
          )
        end

        def render_admin_actions
          div(class: edit_action_classes, data: { testid: 'schedule-edit-action' }) do
            m3_link(
              href: edit_person_schedule_path(person, schedule),
              variant: :outlined,
              size: :lg,
              class: 'h-14 w-full rounded-3xl border-outline/50 bg-surface-container-lowest px-4 ' \
                     'text-lg font-bold hover:bg-surface-container-high transition-all',
              data: { turbo_frame: 'modal', testid: "edit-schedule-#{schedule.id}" }
            ) do
              render Icons::Pencil.new(size: 20, class: 'mr-2 shrink-0')
              plain t('schedules.card.edit', default: 'Edit')
            end
          end
          div(class: delete_action_classes, data: { testid: 'schedule-delete-action' }) do
            render_delete_dialog
          end
        end

        def administrator?
          admin_candidate = current_user
          admin_candidate ||= view_context.current_user if view_context.respond_to?(:current_user)
          admin_candidate&.administrator?
        end

        def render_delete_dialog
          AlertDialog do
            AlertDialogTrigger do
              m3_button(variant: :text,
                        class: 'h-14 w-14 rounded-full bg-transparent p-0 text-error ' \
                               'hover:bg-error-container transition-all',
                        data: { testid: "delete-schedule-#{schedule.id}" }) do
                span(class: 'sr-only') { t('schedules.card.delete', default: 'Delete schedule') }
                render Icons::Trash.new(size: 20)
              end
            end
            AlertDialogContent(class: 'rounded-[2rem] border-none shadow-elevation-5 bg-surface-container-high') do
              AlertDialogHeader do
                AlertDialogTitle { t('schedules.card.delete_dialog.title') }
                AlertDialogDescription do
                  plain t('schedules.card.delete_dialog.confirm', medication: schedule.medication.display_name)
                end
              end
              AlertDialogFooter do
                AlertDialogCancel(class: 'rounded-xl') { t('schedules.card.delete_dialog.cancel') }
                form_with(
                  url: person_schedule_path(person, schedule),
                  method: :delete,
                  class: 'inline'
                ) do
                  m3_button(variant: :destructive, type: :submit, class: 'rounded-xl shadow-lg shadow-error/20') do
                    t('schedules.card.delete_dialog.submit')
                  end
                end
              end
            end
          end
        end
      end
    end
  end
end
