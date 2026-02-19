# frozen_string_literal: true

module Components
  module PersonMedicines
    # Renders a person medicine card with take medicine functionality
    class Card < Components::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::ButtonTo

      attr_reader :person_medicine, :person, :todays_takes, :current_user

      def initialize(person_medicine:, person:, todays_takes: nil, current_user: nil)
        @person_medicine = person_medicine
        @person = person
        @todays_takes = todays_takes
        @current_user = current_user
        super()
      end

      def view_template
        render RubyUI::Card.new(
          id: "person_medicine_#{person_medicine.id}",
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
            render_medicine_icon
            render Components::Shared::StockBadge.new(medicine: person_medicine.medicine)
          end
          div do
            CardTitle(class: 'text-2xl font-black tracking-tight mb-1 text-slate-900') { person_medicine.medicine.name }
            CardDescription(class: 'text-slate-500 font-bold uppercase text-[10px] tracking-widest') do
              medicine_description
            end
          end
        end
      end

      def render_card_content
        CardContent(class: 'flex-grow space-y-6 px-8') do
          div(class: 'pt-4 border-t border-slate-50 space-y-4') do
            render_notes if person_medicine.notes.present?
            render_timing_restrictions if person_medicine.timing_restrictions?
            render_countdown_notice if !person_medicine.can_take_now? && person_medicine.countdown_display
            render_takes_section
          end
        end
      end

      def render_card_footer
        CardFooter(class: 'px-8 pb-8 pt-2') do
          render_person_medicine_actions
        end
      end

      def medicine_description
        parts = []
        if person_medicine.medicine.dosage_amount
          parts << "#{person_medicine.medicine.dosage_amount.to_i}#{person_medicine.medicine.dosage_unit}"
        end
        parts << 'Ad-hoc'
        parts.join(' • ')
      end

      def render_medicine_icon
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
              t('person_medicines.card.notes')
            end
          end
          Text(size: '2', class: 'text-blue-800 leading-relaxed') { person_medicine.notes }
        end
      end

      def render_timing_restrictions
        div(class: 'p-4 bg-slate-50 border border-slate-100 rounded-2xl') do
          div(class: 'flex items-center gap-2 mb-2') do
            render Icons::Settings.new(size: 14, class: 'text-slate-400')
            Text(size: '1', weight: 'black', class: 'uppercase tracking-widest text-slate-400') do
              t('person_medicines.card.timing_restrictions')
            end
          end
          ul(class: 'space-y-1.5') do
            if person_medicine.max_daily_doses.present?
              li(class: 'flex items-center gap-2') do
                div(class: 'w-1 h-1 rounded-full bg-slate-300')
                Text(size: '2', weight: 'semibold', class: 'text-slate-600') do
                  t('person_medicines.card.max_doses_per_day', count: person_medicine.max_daily_doses)
                end
              end
            end
            if person_medicine.min_hours_between_doses.present?
              li(class: 'flex items-center gap-2') do
                div(class: 'w-1 h-1 rounded-full bg-slate-300')
                Text(size: '2', weight: 'semibold', class: 'text-slate-600') do
                  t('person_medicines.card.wait_hours', hours: person_medicine.min_hours_between_doses)
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
              t('person_medicines.card.next_dose_available')
            end
          end
          Text(size: '2', class: 'text-amber-800 font-bold') { person_medicine.countdown_display }
        end
      end

      def render_takes_section
        takes = todays_takes || person_medicine.medication_takes
                                               .where(taken_at: Time.current.beginning_of_day..)
                                               .order(taken_at: :desc)
                                               .load

        div(class: 'space-y-4 pt-2') do
          div(class: 'flex items-center justify-between') do
            Text(size: '1', weight: 'black', class: 'uppercase tracking-widest text-slate-400') do
              t('person_medicines.card.todays_doses')
            end
            render_dose_counter(takes) if person_medicine.max_daily_doses.present?
          end
          render_todays_takes(takes)
        end
      end

      def render_dose_counter(todays_takes)
        todays_count = todays_takes.length
        max_doses = person_medicine.max_daily_doses

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
            t('person_medicines.card.no_doses_today')
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
            Text(size: '2', weight: 'bold', class: 'text-slate-700') { take.taken_at.strftime('%l:%M %p').strip }
          end
          if take.amount_ml.present?
            Text(size: '1', weight: 'black', class: 'text-slate-400 uppercase tracking-tighter') do
              "#{take.amount_ml.to_i}ml"
            end
          end
        end
      end

      def render_take_medicine_button
        return unless view_context.policy(person_medicine).take_medicine?

        if person_medicine.can_administer?
          form_with(
            url: take_medicine_person_person_medicine_path(person, person_medicine),
            method: :post,
            class: 'flex-1',
            data: { controller: 'optimistic-take', action: 'submit->optimistic-take#submit' }
          ) do
            render RubyUI::Button.new(
              type: :submit,
              variant: :primary,
              size: :lg,
              class: 'w-full rounded-xl py-6 font-bold shadow-lg shadow-primary/20 hover:shadow-xl ' \
                     'hover:shadow-primary/30',
              data: { optimistic_take_target: 'button', testid: "take-person-medicine-#{person_medicine.id}" }
            ) do
              plain take_label
            end
          end
        else
          render_disabled_button_with_reason
        end
      end

      def render_disabled_button_with_reason
        reason = person_medicine.administration_blocked_reason
        label = reason == :out_of_stock ? t('person_medicines.card.out_of_stock') : take_label
        render Button.new(
          variant: :secondary,
          size: :lg,
          disabled: true,
          class: 'flex-1 rounded-xl py-6 opacity-50 grayscale',
          data: { testid: "take-person-medicine-#{person_medicine.id}-disabled" }
        ) { label }
      end

      def own_dose?
        return true if current_user.nil?

        current_user.person == person
      end

      def take_label
        own_dose? ? t('person_medicines.card.take') : t('person_medicines.card.give')
      end

      def render_person_medicine_actions
        div(class: 'flex items-center gap-2 w-full') do
          render_take_medicine_button if view_context.policy(person_medicine).take_medicine?

          render_delete_dialog if view_context.policy(person_medicine).destroy?
        end
      end

      def render_delete_dialog
        AlertDialog do
          AlertDialogTrigger do
            Button(
              variant: :ghost,
              class: 'w-12 h-12 p-0 rounded-xl text-slate-300 hover:text-destructive hover:bg-destructive/5',
              data: { testid: "delete-person-medicine-#{person_medicine.id}" }
            ) do
              render Icons::Trash.new(size: 20)
            end
          end
          AlertDialogContent(class: 'rounded-[2rem] border-none shadow-2xl') do
            AlertDialogHeader do
              AlertDialogTitle { t('person_medicines.card.remove_medicine') }
              AlertDialogDescription do
                plain t('person_medicines.card.remove_confirmation', medicine: person_medicine.medicine.name)
              end
            end
            AlertDialogFooter do
              AlertDialogCancel(class: 'rounded-xl') { t('dashboard.delete_confirmation.cancel') }
              form_with(
                url: person_person_medicine_path(person, person_medicine),
                method: :delete,
                class: 'inline'
              ) do
                Button(
                  variant: :destructive,
                  type: :submit,
                  class: 'rounded-xl shadow-lg shadow-destructive/20'
                ) do
                  t('person_medicines.card.remove')
                end
              end
            end
          end
        end
      end
    end
  end
end
