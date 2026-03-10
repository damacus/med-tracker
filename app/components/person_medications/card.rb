# frozen_string_literal: true

module Components
  module PersonMedications
    # Renders a person medication card with take medication functionality
    class Card < Components::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::ButtonTo

      attr_reader :person_medication, :person, :todays_takes, :matching_medications, :current_user

      def initialize(person_medication:, person:, todays_takes: nil, matching_medications: nil, current_user: nil)
        @person_medication = person_medication
        @person = person
        @todays_takes = todays_takes
        @matching_medications = matching_medications
        @current_user = current_user
        super()
      end

      def view_template
        render RubyUI::Card.new(
          id: "person_medication_#{person_medication.id}",
          class: 'h-full flex flex-col border-none border-l-4 border-l-primary ' \
                 'shadow-[0_8px_30px_rgb(0,0,0,0.06)] bg-white rounded-[2.5rem] transition-all ' \
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
          div do
            CardTitle(class: 'text-2xl font-black tracking-tight mb-1 text-slate-900') do
              person_medication.medication.name
            end
            CardDescription(class: 'text-slate-500 font-bold uppercase text-[10px] tracking-widest') do
              medication_description
            end
          end
        end
      end

      def render_card_content
        CardContent(class: 'flex-grow space-y-6 px-8') do
          div(class: 'pt-4 border-t border-slate-50 space-y-4') do
            render_notes if person_medication.notes.present?
            render_timing_restrictions if person_medication.timing_restrictions?
            render_countdown_notice if !person_medication.can_take_now? && person_medication.countdown_display
            render_takes_section
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
        if person_medication.dose_amount
          parts << "#{person_medication.dose_amount.to_f.to_s.sub(/\.0$/, '')}#{person_medication.dose_unit}"
        end
        parts << 'As needed'
        parts.join(' • ')
      end

      def render_medication_icon
        div(
          class: 'w-12 h-12 rounded-2xl bg-slate-50 flex items-center justify-center text-slate-400 ' \
                 'group-hover:text-primary group-hover:bg-primary/5 transition-all'
        ) do
          render Icons::Pill.new(size: 24)
        end
      end

      def render_notes
        div(class: 'p-4 bg-blue-50/50 border border-blue-100 rounded-2xl') do
          div(class: 'flex items-center gap-2 mb-1') do
            render Icons::AlertCircle.new(size: 14, class: 'text-blue-600')
            Text(size: '1', weight: 'black', class: 'uppercase tracking-widest text-blue-600') do
              t('person_medications.card.notes')
            end
          end
          Text(size: '2', class: 'text-blue-800 leading-relaxed') { person_medication.notes }
        end
      end

      def render_timing_restrictions
        div(class: 'p-4 bg-slate-50 border border-slate-100 rounded-2xl') do
          div(class: 'flex items-center gap-2 mb-2') do
            render Icons::Settings.new(size: 14, class: 'text-slate-400')
            Text(size: '1', weight: 'black', class: 'uppercase tracking-widest text-slate-400') do
              t('person_medications.card.timing_restrictions')
            end
          end
          ul(class: 'space-y-1.5') do
            if person_medication.max_daily_doses.present?
              li(class: 'flex items-center gap-2') do
                div(class: 'w-1 h-1 rounded-full bg-slate-300')
                Text(size: '2', weight: 'semibold', class: 'text-slate-600') do
                  t('person_medications.card.max_doses_per_day', count: person_medication.max_daily_doses)
                end
              end
            end
            if person_medication.min_hours_between_doses.present?
              li(class: 'flex items-center gap-2') do
                div(class: 'w-1 h-1 rounded-full bg-slate-300')
                Text(size: '2', weight: 'semibold', class: 'text-slate-600') do
                  t('person_medications.card.wait_hours', hours: person_medication.min_hours_between_doses)
                end
              end
            end
          end
        end
      end

      def render_countdown_notice
        div(class: 'p-4 bg-amber-50/50 border border-amber-100 rounded-2xl') do
          div(class: 'flex items-center gap-2 mb-1') do
            render Icons::AlertCircle.new(size: 14, class: 'text-amber-600')
            Text(size: '1', weight: 'black', class: 'uppercase tracking-widest text-amber-600') do
              t('person_medications.card.next_dose_available')
            end
          end
          Text(size: '2', class: 'text-amber-800 font-bold') { person_medication.countdown_display }
        end
      end

      def render_takes_section
        takes = todays_takes || fetch_todays_takes

        div(class: 'space-y-4 pt-2') do
          div(class: 'flex items-center justify-between') do
            Text(size: '1', weight: 'black', class: 'uppercase tracking-widest text-slate-400') do
              t('person_medications.card.todays_doses')
            end
            render_dose_counter(takes) if person_medication.max_daily_doses.present?
          end
          render_todays_takes(takes)
        end
      end

      def fetch_todays_takes
        if person_medication.medication_takes.loaded?
          person_medication.medication_takes
                           .select { |t| t.taken_at >= Time.current.beginning_of_day }
                           .sort_by(&:taken_at)
                           .reverse
        else
          person_medication.medication_takes
                           .where(taken_at: Time.current.beginning_of_day..)
                           .order(taken_at: :desc)
                           .load
        end
      end

      def render_dose_counter(todays_takes)
        todays_count = todays_takes.length
        max_doses = person_medication.max_daily_doses

        Badge(variant: :outline, class: 'rounded-full text-[10px]') do
          "#{todays_count}/#{max_doses}"
        end
      end

      def render_todays_takes(todays_takes)
        if todays_takes.any?
          div(class: 'grid grid-cols-1 gap-2') do
            todays_takes.each do |take|
              render_take_item(take)
            end
          end
        else
          Text(size: '2', weight: 'medium', class: 'italic text-slate-300 px-1') do
            t('person_medications.card.no_doses_today')
          end
        end
      end

      def render_take_item(take)
        div(
          class: 'flex items-center justify-between p-3 rounded-xl bg-slate-50/50 group/item transition-colors ' \
                 'hover:bg-slate-50'
        ) do
          div(class: 'flex items-center gap-3') do
            render Icons::CheckCircle.new(size: 16, class: 'text-emerald-500')
            div(class: 'space-y-1') do
              Text(size: '2', weight: 'bold', class: 'text-slate-700') { take.taken_at.strftime('%l:%M %p').strip }
              if take.inventory_location.present?
                Text(size: '1', class: 'text-slate-400') { take.inventory_location.name }
              end
            end
          end
          if take.amount_ml.present?
            Text(size: '1', weight: 'black', class: 'text-slate-400 uppercase tracking-tighter') do
              "#{take.amount_ml.to_f.to_s.sub(/\.0$/, '')}#{person_medication.dose_unit}"
            end
          end
        end
      end

      def render_take_medication_button
        return unless view_context.policy(person_medication).take_medication?

        label = if invalid_dose_configured?
                  t('person_medications.card.invalid_dose')
                else
                  blocked_reason == :out_of_stock ? t('person_medications.card.out_of_stock') : take_label
                end
        render Components::Medications::TakeAction.new(
          source: person_medication,
          context: { person: person, current_user: current_user },
          amount: person_medication.dose_amount,
          button: {
            label: take_label,
            variant: :primary,
            size: :lg,
            class: 'w-full rounded-xl py-6 font-bold shadow-lg shadow-primary/20 hover:shadow-xl ' \
                   'hover:shadow-primary/30',
            testid: "take-person-medication-#{person_medication.id}",
            form_class: 'flex-1'
          },
          state: {
            disabled: invalid_dose_configured? || blocked_reason.present?,
            label: label
          }
        )
      end

      def own_dose?
        return true if current_user.nil?

        current_user.person == person
      end

      def take_label
        own_dose? ? t('person_medications.card.take') : t('person_medications.card.give')
      end

      def invalid_dose_configured?
        person_medication.dose_amount.to_f <= 0
      end

      # Cache the resolved blocked reason per render so a nil result does not
      # re-run stock resolution for both the disabled label and disabled state.
      def blocked_reason
        return @blocked_reason if instance_variable_defined?(:@blocked_reason)

        @blocked_reason = stock_source_resolver.blocked_reason
      end

      def stock_source_resolver
        @stock_source_resolver ||= MedicationStockSourceResolver.new(
          user: current_user,
          source: person_medication,
          matching_medications: matching_medications
        )
      end

      def render_person_medication_actions
        div(class: 'flex items-center gap-2 w-full') do
          render_reorder_controls if view_context.policy(person_medication).update?
          render_edit_button if view_context.policy(person_medication).update?

          render_take_medication_button if view_context.policy(person_medication).take_medication?

          render_delete_dialog if view_context.policy(person_medication).destroy?
        end
      end

      def render_edit_button
        a(
          href: edit_person_person_medication_path(person, person_medication),
          data: { turbo_frame: 'modal', testid: "edit-person-medication-#{person_medication.id}" },
          class: 'inline-flex items-center justify-center w-10 h-10 rounded-xl text-slate-400 ' \
                 'hover:text-slate-700 hover:bg-slate-100 transition-colors',
          aria_label: t('person_medications.card.edit')
        ) do
          render Icons::Pencil.new(size: 16)
        end
      end

      def render_reorder_controls
        div(class: 'flex items-center gap-1') do
          form_with(
            url: reorder_person_person_medication_path(person, person_medication),
            method: :patch,
            class: 'inline'
          ) do
            input(type: :hidden, name: :direction, value: 'up')
            Button(
              variant: :ghost,
              type: :submit,
              class: 'w-10 h-10 p-0 rounded-xl text-slate-400 hover:text-slate-700',
              data: { testid: "move-up-person-medication-#{person_medication.id}" },
              aria_label: t('person_medications.card.move_up_aria_label')
            ) do
              render Icons::ArrowUp.new(size: 16)
            end
          end

          form_with(
            url: reorder_person_person_medication_path(person, person_medication),
            method: :patch,
            class: 'inline'
          ) do
            input(type: :hidden, name: :direction, value: 'down')
            Button(
              variant: :ghost,
              type: :submit,
              class: 'w-10 h-10 p-0 rounded-xl text-slate-400 hover:text-slate-700',
              data: { testid: "move-down-person-medication-#{person_medication.id}" },
              aria_label: t('person_medications.card.move_down_aria_label')
            ) do
              render Icons::ArrowDown.new(size: 16)
            end
          end
        end
      end

      def render_delete_dialog
        AlertDialog do
          AlertDialogTrigger do
            Button(
              variant: :ghost,
              class: 'w-12 h-12 p-0 rounded-xl text-slate-300 hover:text-destructive hover:bg-destructive/5',
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
                plain t('person_medications.card.remove_confirmation', medication: person_medication.medication.name)
              end
            end
            AlertDialogFooter do
              AlertDialogCancel(class: 'rounded-xl') { t('dashboard.delete_confirmation.cancel') }
              form_with(
                url: person_person_medication_path(person, person_medication),
                method: :delete,
                class: 'inline'
              ) do
                Button(
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
