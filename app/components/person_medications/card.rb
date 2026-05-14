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
          id: "person_medication_#{person_medication.id}",
          class: 'h-full flex flex-col border-none border-l-4 border-l-primary ' \
                 'shadow-[0_8px_30px_rgb(0,0,0,0.06)] bg-card rounded-[2.5rem] transition-all ' \
                 'duration-300 hover:scale-[1.02] hover:shadow-xl group overflow-hidden'
        ) do
          render_card_header
          render_card_content
          render_card_footer
        end
      end

      private

      def render_card_header
        CardHeader(class: 'pb-4 pt-8 px-8') do
          div(class: 'flex justify-between items-start mb-4') do
            render_medication_icon
            render Components::Shared::StockBadge.new(medication: person_medication.medication)
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

      def render_card_content
        CardContent(class: 'flex-grow space-y-6 px-8') do
          div(class: 'pt-4 border-t border-border space-y-4') do
            render_notes if person_medication.notes.present?
            render_timing_restrictions if person_medication.timing_restrictions?
          end
        end
      end

      def render_card_footer
        CardFooter(class: 'px-8 pb-8 pt-2') do
          render_person_medication_actions
        end
      end

      def medication_description
        parts = []
        dose = DoseAmount.new(person_medication.dose_amount, person_medication.dose_unit).label
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
        div(class: 'flex items-center gap-2 w-full') do
          render_reorder_controls if view_context.policy(person_medication).update?
          render_past_dose_button
          render_edit_button if view_context.policy(person_medication).update?
          render_delete_dialog if view_context.policy(person_medication).destroy?
        end
      end

      def render_past_dose_button
        render Components::Medications::PriorDayTakeAction.new(
          source: person_medication,
          context: { person: person, current_user: current_user },
          amount: person_medication.dose_amount,
          testid: "log-past-dose-person-medication-#{person_medication.id}",
          button: {
            variant: :outlined,
            size: :lg,
            class: 'flex-1 rounded-xl py-6 font-bold border-outline text-on-surface-variant ' \
                   'hover:bg-tertiary-container transition-colors'
          }
        )
      end

      def render_edit_button
        a(
          href: edit_person_person_medication_path(person, person_medication),
          data: { turbo_frame: 'modal', testid: "edit-person-medication-#{person_medication.id}" },
          class: 'inline-flex items-center justify-center w-12 min-w-12 h-12 shrink-0 rounded-xl ' \
                 'text-on-surface-variant ' \
                 'border border-outline hover:text-foreground hover:bg-tertiary-container transition-colors',
          aria_label: t('person_medications.card.edit')
        ) do
          render Icons::Pencil.new(size: 20)
        end
      end

      def render_reorder_controls
        div(class: 'flex flex-col gap-1') do
          form_with(
            url: reorder_person_person_medication_path(person, person_medication),
            method: :patch,
            class: 'inline'
          ) do
            input(type: :hidden, name: :direction, value: 'up')
            m3_button(
              variant: :text,
              type: :submit,
              class: 'w-8 h-6 p-0 rounded-md text-on-surface-variant hover:text-foreground',
              data: { testid: "move-up-person-medication-#{person_medication.id}" },
              aria_label: t('person_medications.card.move_up_aria_label')
            ) do
              render Icons::ArrowUp.new(size: 14)
            end
          end

          form_with(
            url: reorder_person_person_medication_path(person, person_medication),
            method: :patch,
            class: 'inline'
          ) do
            input(type: :hidden, name: :direction, value: 'down')
            m3_button(
              variant: :text,
              type: :submit,
              class: 'w-8 h-6 p-0 rounded-md text-on-surface-variant hover:text-foreground',
              data: { testid: "move-down-person-medication-#{person_medication.id}" },
              aria_label: t('person_medications.card.move_down_aria_label')
            ) do
              render Icons::ArrowDown.new(size: 14)
            end
          end
        end
      end

      def render_delete_dialog
        AlertDialog do
          AlertDialogTrigger do
            m3_button(
              variant: :text,
              class: 'w-12 min-w-12 h-12 shrink-0 p-0 rounded-xl text-on-surface-variant ' \
                     'hover:text-destructive hover:bg-destructive/5',
              data: { testid: "delete-person-medication-#{person_medication.id}" },
              aria_label: t('person_medications.card.delete_aria_label')
            ) do
              render Icons::Trash.new(size: 20)
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
    end
  end
end
