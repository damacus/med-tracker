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
            div(class: 'flex items-center gap-2 w-full') do
              render_take_medication_button
              render_overflow_menu
              render_admin_actions if administrator?
            end
          end
        end

        private

        def render_overflow_menu
          render RubyUI::DropdownMenu.new do
            render RubyUI::DropdownMenuTrigger.new do
              m3_button(
                variant: :text,
                class: 'w-12 h-12 p-0 rounded-xl text-on-surface-variant hover:text-foreground transition-all',
                data: { testid: "more-actions-schedule-#{schedule.id}" },
                aria_label: t('medications.card.more_actions', default: 'More actions')
              ) do
                render Icons::MoreHorizontal.new(size: 20)
              end
            end
            render RubyUI::DropdownMenuContent.new(class: 'w-56') do
              render Components::Medications::PriorDayTakeAction.new(
                source: schedule,
                context: { person: person, current_user: current_user },
                amount: schedule.dose_amount,
                testid: "log-past-dose-schedule-#{schedule.id}"
              )
            end
          end
        end

        def render_take_medication_button
          render Components::Medications::TakeAction.new(
            source: schedule,
            context: { person: person, current_user: current_user },
            amount: schedule.dose_amount,
            button: {
              label: presenter.take_label,
              variant: :filled,
              size: :lg,
              icon: Icons::Pill,
              class: 'w-full rounded-xl py-6 font-bold shadow-lg shadow-primary/20 hover:shadow-xl ' \
                     'hover:shadow-primary/30 transition-all',
              testid: "take-schedule-#{schedule.id}",
              form_class: 'flex-1'
            },
            state: {
              disabled: presenter.take_disabled?,
              label: presenter.take_state_label,
              icon: Icons::AlertCircle
            }
          )
        end

        def render_admin_actions
          m3_link(
            href: edit_person_schedule_path(person, schedule),
            variant: :outlined,
            size: :lg,
            class: 'w-12 h-12 p-0 rounded-xl border-outline text-on-surface-variant ' \
                   'hover:text-primary hover:border-primary/50 transition-all',
            data: { turbo_frame: 'modal', testid: "edit-schedule-#{schedule.id}" }
          ) do
            span(class: 'sr-only') { t('schedules.card.edit', default: 'Edit schedule') }
            render Icons::Pencil.new(size: 20)
          end
          render_delete_dialog
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
                        class: 'w-12 h-12 p-0 rounded-xl text-on-surface-variant ' \
                               'hover:text-error hover:bg-error/5 transition-all',
                        data: { testid: "delete-schedule-#{schedule.id}" }) do
                span(class: 'sr-only') { t('schedules.card.delete', default: 'Delete schedule') }
                render Icons::Trash.new(size: 20)
              end
            end
            AlertDialogContent(class: 'rounded-[2rem] border-none shadow-elevation-5 bg-surface-container-high') do
              AlertDialogHeader do
                AlertDialogTitle { t('schedules.card.delete_dialog.title') }
                AlertDialogDescription do
                  plain t('schedules.card.delete_dialog.confirm', medication: schedule.medication.name)
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
