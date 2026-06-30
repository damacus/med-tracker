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
            div(class: 'flex min-w-0 items-center gap-2 w-full',
                data: { testid: 'schedule-card-actions' }) do
              render_past_dose_button unless schedule.paused?
              render_actions_menu if administrator?
            end
          end
        end

        private

        def render_past_dose_button
          div(class: 'min-w-0 flex-1') do
            render Components::Medications::PriorDayTakeAction.new(
              source: schedule,
              context: { person: person, current_user: current_user },
              amount: schedule.dose_amount,
              testid: "log-past-dose-schedule-#{schedule.id}",
              button: {
                variant: :outlined,
                size: :lg,
                class: 'w-full min-w-0 overflow-hidden rounded-shape-full px-3 font-bold ' \
                       'border-outline text-on-surface-variant hover:bg-surface-container-high transition-all'
              }
            )
          end
        end

        def render_actions_menu
          render RubyUI::DropdownMenu.new(options: { placement: 'bottom-end' }, class: 'shrink-0') do
            render RubyUI::DropdownMenuTrigger.new do
              m3_button(
                variant: :outlined,
                type: :button,
                class: 'shrink-0 rounded-shape-full border-outline px-3 text-on-surface-variant ' \
                       'hover:bg-tertiary-container hover:text-foreground transition-colors',
                data: { testid: "schedule-actions-#{schedule.id}" },
                aria_label: t('schedules.card.actions')
              ) do
                render Icons::MoreHorizontal.new(size: 18, aria_hidden: 'true', class: 'mr-2 shrink-0')
                plain t('schedules.card.actions')
              end
            end
            render RubyUI::DropdownMenuContent.new(
              class: 'w-56 rounded-shape-xl border border-border/70 bg-popover p-1 shadow-elevation-3',
              data: { testid: "schedule-actions-menu-#{schedule.id}" }
            ) do
              render_active_state_action
              render_edit_action
              render_delete_dialog
            end
          end
        end

        def render_edit_action
          a(
            href: edit_person_schedule_path(person, schedule),
            data: { turbo_frame: 'modal', testid: "edit-schedule-#{schedule.id}" },
            class: menu_item_class,
            role: 'menuitem',
            data_action: 'click->ruby-ui--dropdown-menu#close',
            data_ruby_ui__dropdown_menu_target: 'menuItem',
            tabindex: '-1'
          ) do
            render Icons::Pencil.new(size: 16, aria_hidden: 'true', class: 'mr-2 shrink-0')
            span { t('schedules.card.edit', default: 'Edit schedule') }
          end
        end

        def render_active_state_action
          form_with(
            url: active_state_path,
            method: :patch,
            class: 'contents'
          ) do
            button(
              type: :submit,
              role: 'menuitem',
              class: menu_item_class,
              data: { testid: active_state_testid },
              data_action: 'click->ruby-ui--dropdown-menu#close',
              data_ruby_ui__dropdown_menu_target: 'menuItem',
              tabindex: '-1'
            ) do
              render active_state_icon.new(size: 16, aria_hidden: 'true', class: 'mr-2 shrink-0')
              span { active_state_label }
            end
          end
        end

        def active_state_path
          if schedule.paused?
            resume_person_schedule_path(person, schedule)
          else
            pause_person_schedule_path(person, schedule)
          end
        end

        def active_state_label
          if schedule.paused?
            t('schedules.card.resume')
          else
            t('schedules.card.pause')
          end
        end

        def active_state_testid
          "#{schedule.paused? ? 'resume' : 'pause'}-schedule-#{schedule.id}"
        end

        def active_state_icon
          schedule.paused? ? Icons::RefreshCw : Icons::Clock
        end

        def administrator?
          admin_candidate = current_user
          admin_candidate ||= view_context.current_user if view_context.respond_to?(:current_user)
          SchedulePolicy.new(policy_context(admin_candidate), schedule).update?
        end

        def render_delete_dialog
          AlertDialog do
            AlertDialogTrigger do
              button(
                type: :button,
                role: 'menuitem',
                class: "#{menu_item_class} text-destructive hover:bg-destructive/5 hover:text-destructive",
                data: { testid: "delete-schedule-#{schedule.id}" },
                data_action: 'click->ruby-ui--dropdown-menu#close',
                data_ruby_ui__dropdown_menu_target: 'menuItem',
                tabindex: '-1'
              ) do
                render Icons::Trash.new(size: 16, aria_hidden: 'true', class: 'mr-2 shrink-0')
                span { t('schedules.card.delete', default: 'Delete schedule') }
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
                AlertDialogCancel { t('schedules.card.delete_dialog.cancel') }
                form_with(
                  url: person_schedule_path(person, schedule),
                  method: :delete,
                  class: 'inline'
                ) do
                  m3_button(variant: :destructive, type: :submit, class: 'shadow-elevation-2') do
                    t('schedules.card.delete_dialog.submit')
                  end
                end
              end
            end
          end
        end

        def menu_item_class
          'relative flex w-full cursor-pointer select-none items-center rounded-shape-sm px-3 py-2 text-sm ' \
            'text-on-surface-variant outline-none transition-colors hover:bg-tertiary-container ' \
            'hover:text-on-tertiary-container focus:bg-tertiary-container focus:text-on-tertiary-container'
        end
      end
    end
  end
end
