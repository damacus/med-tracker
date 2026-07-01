# frozen_string_literal: true

module Components
  module PersonMedications
    # Renders a person medication card with take medication functionality
    class Card < Components::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::ButtonTo

      attr_reader :person_medication, :person, :current_user

      def initialize(person_medication:, person:, current_user: nil)
        @person_medication = person_medication
        @person = person
        @current_user = current_user
        super()
      end

      def view_template
        render M3::Card.new(
          id: tenant_dom_id(person_medication),
          class: card_class
        ) do
          render_card_header
          render_card_content
          render_card_footer
        end
      end

      private

      def card_class
        base = 'h-full flex flex-col border-none border-l-4 border-l-primary ' \
               'shadow-[0_8px_30px_rgb(0,0,0,0.06)] bg-card rounded-[2.5rem] transition-all ' \
               'duration-300 group overflow-hidden'
        return "#{base} opacity-70 grayscale-[0.2]" if person_medication.paused?

        "#{base} hover:scale-[1.02] hover:shadow-xl"
      end

      def render_card_header
        CardHeader(class: 'pb-4 pt-8 px-8') do
          div(class: 'flex justify-between items-start mb-4') do
            render_medication_icon
            div(class: 'flex flex-col items-end gap-2 shrink-0') do
              render_paused_badge if person_medication.paused?
              render Components::Shared::StockBadge.new(medication: person_medication.medication)
            end
          end
          div(class: 'min-w-0') do
            CardTitle(class: 'text-2xl font-black tracking-tight mb-1 text-foreground break-words leading-tight') do
              person_medication.medication.display_name
            end
            CardDescription(class: 'text-on-surface-variant font-bold uppercase text-[10px] tracking-widest') do
              medication_description
            end
          end
        end
      end

      def render_paused_badge
        m3_badge(variant: :outlined, class: 'rounded-full uppercase text-[10px] font-black tracking-widest') do
          t('person_medications.card.paused')
        end
      end

      def render_card_content
        CardContent(class: 'flex-grow space-y-6 px-8') do
          div(class: 'pt-4 border-t border-border space-y-4') do
            render_notes if person_medication.notes.present?
            render_timing_restrictions if person_medication.timing_restrictions?
          end
        end
      end

      def render_card_footer
        CardFooter(class: 'px-6 pb-8 pt-2') do
          render_person_medication_actions
        end
      end

      def medication_description
        parts = []
        dose = DoseAmount.new(person_medication.dose_amount, person_medication.dose_unit).to_s
        parts << dose if dose.present?
        parts << t('people.add_medication.otc_title')
        parts.join(' • ')
      end

      def render_medication_icon
        div(
          class: 'w-12 h-12 rounded-shape-xl bg-secondary-container flex items-center justify-center ' \
                 'text-on-surface-variant ' \
                 'group-hover:text-primary group-hover:bg-primary/5 transition-all'
        ) do
          render Components::Shared::MedicationIcon.new(medication: person_medication.medication, size: 24)
        end
      end

      def render_notes
        div(class: 'p-4 bg-primary-container border border-primary/20 rounded-shape-xl') do
          div(class: 'flex items-center gap-2 mb-1') do
            render Icons::FileText.new(size: 14, class: 'text-on-primary-container')
            m3_text(size: '1', weight: 'bold',
                    class: 'font-black uppercase tracking-widest text-on-primary-container') do
              t('person_medications.card.notes')
            end
          end
          m3_text(size: '2', class: 'text-on-primary-container leading-relaxed') { person_medication.notes }
        end
      end

      def render_timing_restrictions
        div(class: 'p-4 bg-secondary-container border border-border rounded-shape-xl') do
          div(class: 'flex items-center gap-2 mb-2') do
            render Icons::Clock.new(size: 14, class: 'text-on-surface-variant')
            m3_text(size: '1', weight: 'bold', class: 'font-black uppercase tracking-widest text-on-surface-variant') do
              t('person_medications.card.timing_restrictions')
            end
          end
          ul(class: 'space-y-1.5') do
            if person_medication.max_daily_doses.present?
              li(class: 'flex items-center gap-2') do
                div(class: 'w-1 h-1 rounded-full bg-secondary-container-foreground')
                m3_text(size: '2', weight: 'semibold', class: 'text-on-surface-variant') do
                  t('person_medications.card.max_doses_per_day', count: person_medication.max_daily_doses)
                end
              end
            end
            if person_medication.min_hours_between_doses.present?
              li(class: 'flex items-center gap-2') do
                div(class: 'w-1 h-1 rounded-full bg-secondary-container-foreground')
                m3_text(size: '2', weight: 'semibold', class: 'text-on-surface-variant') do
                  t('person_medications.card.wait_hours', hours: person_medication.min_hours_between_doses)
                end
              end
            end
          end
        end
      end

      def render_person_medication_actions
        div(class: 'flex min-w-0 items-center gap-0.5 w-full',
            data: { testid: 'person-medication-card-actions' }) do
          render_past_dose_button unless person_medication.paused?
          render_actions_menu if secondary_actions?
        end
      end

      def secondary_actions?
        view_context.policy(person_medication).update? || view_context.policy(person_medication).destroy?
      end

      def render_past_dose_button
        div(class: 'min-w-0 flex-1') do
          render Components::Medications::PriorDayTakeAction.new(
            source: person_medication,
            context: { person: person, current_user: current_user },
            amount: person_medication.dose_amount,
            testid: "log-past-dose-person-medication-#{person_medication.id}",
            button: {
              variant: :outlined,
              size: :lg,
              trigger_class: 'block w-full',
              class: 'w-full min-w-0 state-layer-overflow-visible rounded-shape-full font-bold ' \
                     'border-outline text-on-surface-variant hover:bg-tertiary-container transition-colors'
            }
          )
        end
      end

      def render_actions_menu
        render RubyUI::DropdownMenu.new(options: { placement: 'bottom-end', strategy: 'fixed' }, class: 'shrink-0') do
          render RubyUI::DropdownMenuTrigger.new(class: 'shrink-0') do
            render RubyUI::Button.new(
              variant: :outline,
              type: :button,
              class: 'shrink-0',
              data: { testid: "person-medication-actions-#{person_medication.id}" },
              aria_label: t('person_medications.card.actions')
            ) do
              t('person_medications.card.actions')
            end
          end
          render RubyUI::DropdownMenuContent.new(
            class: 'w-56 max-w-[calc(100vw-2rem)] rounded-shape-xl border border-border/70 bg-popover p-1 ' \
                   'shadow-elevation-3',
            data: { testid: "person-medication-actions-menu-#{person_medication.id}" }
          ) do
            if view_context.policy(person_medication).update?
              render_reorder_controls
              render_active_state_button
              render_edit_button
            end
            render_delete_dialog if view_context.policy(person_medication).destroy?
          end
        end
      end

      def render_active_state_button
        form_with(
          url: active_state_path,
          method: :patch,
          class: 'contents'
        ) do
          render RubyUI::DropdownMenuItem.new(
            as: :button,
            type: :submit,
            class: menu_item_class,
            data: { testid: active_state_testid }
          ) do
            render active_state_icon.new(size: 16, aria_hidden: 'true', class: 'mr-2 shrink-0')
            span { active_state_label }
          end
        end
      end

      def active_state_path
        if person_medication.paused?
          resume_person_person_medication_path(person, person_medication)
        else
          pause_person_person_medication_path(person, person_medication)
        end
      end

      def active_state_label
        if person_medication.paused?
          t('person_medications.card.resume')
        else
          t('person_medications.card.pause')
        end
      end

      def active_state_testid
        "#{person_medication.paused? ? 'resume' : 'pause'}-person-medication-#{person_medication.id}"
      end

      def active_state_icon
        person_medication.paused? ? Icons::RefreshCw : Icons::Clock
      end

      def render_edit_button
        render RubyUI::DropdownMenuItem.new(
          href: edit_person_person_medication_path(person, person_medication),
          data: { turbo_frame: 'modal', testid: "edit-person-medication-#{person_medication.id}" },
          class: menu_item_class
        ) do
          render Icons::Pencil.new(size: 16, aria_hidden: 'true', class: 'mr-2 shrink-0')
          span { t('person_medications.card.edit') }
        end
      end

      def render_reorder_controls
        render_reorder_menu_item(direction: 'up', icon: Icons::ArrowUp, label: t('person_medications.card.move_up'),
                                 testid: "move-up-person-medication-#{person_medication.id}")
        render_reorder_menu_item(direction: 'down', icon: Icons::ArrowDown,
                                 label: t('person_medications.card.move_down'),
                                 testid: "move-down-person-medication-#{person_medication.id}")
      end

      def render_reorder_menu_item(direction:, icon:, label:, testid:)
        form_with(
          url: reorder_person_person_medication_path(person, person_medication),
          method: :patch,
          class: 'contents'
        ) do
          input(type: :hidden, name: :direction, value: direction)
          render RubyUI::DropdownMenuItem.new(
            as: :button,
            type: :submit,
            class: menu_item_class,
            data: { testid: testid }
          ) do
            render icon.new(size: 16, aria_hidden: 'true', class: 'mr-2 shrink-0')
            span { label }
          end
        end
      end

      def render_delete_dialog
        AlertDialog(class: 'block w-full') do
          AlertDialogTrigger(class: 'block w-full') do
            render RubyUI::DropdownMenuItem.new(
              as: :button,
              type: :button,
              class: "#{menu_item_class} text-destructive hover:bg-destructive/5 hover:text-destructive",
              data: { testid: "delete-person-medication-#{person_medication.id}" }
            ) do
              render Icons::Trash.new(size: 16, aria_hidden: 'true', class: 'mr-2 shrink-0')
              span { t('person_medications.card.delete_aria_label') }
            end
          end
          AlertDialogContent(class: 'rounded-[2rem] border-none shadow-2xl') do
            AlertDialogHeader do
              AlertDialogTitle { t('person_medications.card.remove_medication') }
              AlertDialogDescription do
                plain t('person_medications.card.remove_confirmation',
                        medication: person_medication.medication.display_name)
              end
            end
            AlertDialogFooter do
              AlertDialogCancel(class: 'rounded-xl') { t('dashboard.delete_confirmation.cancel') }
              form_with(
                url: person_person_medication_path(person, person_medication),
                method: :delete,
                class: 'inline'
              ) do
                m3_button(
                  variant: :destructive,
                  type: :submit,
                  class: 'rounded-xl shadow-lg shadow-destructive/20'
                ) do
                  t('person_medications.card.remove')
                end
              end
            end
          end
        end
      end

      def menu_item_class
        'w-full rounded-shape-sm px-3 py-2 text-on-surface-variant'
      end
    end
  end
end
