# frozen_string_literal: true

module Components
  module PersonMedicines
    # Renders a person medicine card with take medicine functionality
    class Card < Components::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::ButtonTo
      include Phlex::Rails::Helpers::T

      attr_reader :person_medicine, :person

      def initialize(person_medicine:, person:)
        @person_medicine = person_medicine
        @person = person
        super()
      end

      def view_template
        RubyUI::Card(id: "person_medicine_#{person_medicine.id}", class: 'h-full flex flex-col') do
          render_card_header
          render_card_content
          render_card_footer
        end
      end

      private

      def render_card_header
        CardHeader do
          render_medicine_icon
          CardTitle(class: 'text-xl') { person_medicine.medicine.name }
          CardDescription { medicine_description }
        end
      end

      def render_card_content
        CardContent(class: 'flex-grow space-y-4') do
          render_notes if person_medicine.notes.present?
          render_timing_restrictions if person_medicine.timing_restrictions?
          render_countdown_notice if !person_medicine.can_take_now? && person_medicine.countdown_display
          render_takes_section
        end
      end

      def render_card_footer
        CardFooter(class: 'flex gap-2') do
          render_person_medicine_actions
        end
      end

      def medicine_description
        parts = []
        if person_medicine.medicine.dosage_amount
          parts << "#{person_medicine.medicine.dosage_amount.to_i} #{person_medicine.medicine.dosage_unit}"
        end
        parts.join(' • ')
      end

      def render_medicine_icon
        div(class: 'w-10 h-10 rounded-xl flex items-center justify-center bg-violet-100 text-violet-700 mb-2') do
          render Icons::Pill.new(size: 20)
        end
      end

      def render_notes
        div(class: 'p-3 bg-blue-50 border border-blue-200 rounded-md') do
          Text(size: '2', class: 'text-blue-800') do
            span(class: 'font-semibold') { t('person_medicines.card.notes') }
            plain person_medicine.notes
          end
        end
      end

      def render_timing_restrictions
        div(class: 'p-3 bg-amber-50 border border-amber-200 rounded-md') do
          Text(size: '2', weight: 'semibold', class: 'text-amber-800 mb-1') do
            t('person_medicines.card.timing_restrictions')
          end
          ul(class: 'my-1 ml-4 text-sm text-amber-800 list-disc [&>li]:mt-0.5') do
            if person_medicine.max_daily_doses.present?
              li { t('person_medicines.card.max_doses_per_day', count: person_medicine.max_daily_doses) }
            end
            if person_medicine.min_hours_between_doses.present?
              li { t('person_medicines.card.wait_hours', hours: person_medicine.min_hours_between_doses) }
            end
          end
        end
      end

      def render_countdown_notice
        div(class: 'p-3 bg-amber-50 border border-amber-200 rounded-md') do
          Text(size: '2', class: 'text-amber-800') do
            span(class: 'font-semibold') { t('person_medicines.card.next_dose_available') }
            plain person_medicine.countdown_display
          end
        end
      end

      def render_takes_section
        todays_takes = person_medicine.medication_takes
                                      .where(taken_at: Time.current.beginning_of_day..)
                                      .order(taken_at: :desc)
                                      .load

        div(class: 'space-y-3') do
          div(class: 'flex items-center justify-between') do
            Heading(level: 4, size: '2', class: 'font-semibold text-slate-700') do
              t('person_medicines.card.todays_doses')
            end
            render_dose_counter(todays_takes) if person_medicine.max_daily_doses.present?
          end
          render_todays_takes(todays_takes)
        end
      end

      def render_dose_counter(todays_takes)
        todays_count = todays_takes.length
        max_doses = person_medicine.max_daily_doses

        badge_class = if todays_count >= max_doses
                        'bg-destructive-light text-destructive-text'
                      elsif todays_count.positive?
                        'bg-success-light text-success-text'
                      else
                        'bg-slate-100 text-slate-600'
                      end

        span(class: "text-xs font-medium px-2 py-1 rounded-full min-h-[24px] #{badge_class}") do
          "#{todays_count}/#{max_doses}"
        end
      end

      def render_todays_takes(todays_takes)
        if todays_takes.any?
          div(class: 'space-y-2') do
            todays_takes.each do |take|
              render_take_item(take)
            end
          end
        else
          Text(size: '2', weight: 'muted', class: 'italic') { t('person_medicines.card.no_doses_today') }
        end
      end

      def render_take_item(take)
        div(class: 'flex items-center gap-2 text-sm') do
          render Icons::CheckCircle.new(size: 16, class: 'w-4 h-4 text-success')
          Text(as: 'span', weight: 'medium', class: 'text-slate-700') { take.taken_at.strftime('%l:%M %p').strip }
          Text(as: 'span', class: 'text-slate-500') { "#{take.amount_ml.to_i} ml" } if take.amount_ml.present?
        end
      end

      def render_take_medicine_button
        return unless view_context.policy(person_medicine).take_medicine?

        if person_medicine.can_administer?
          form_with(
            url: take_medicine_person_person_medicine_path(person, person_medicine),
            method: :post,
            class: 'inline-block',
            data: { controller: 'optimistic-take', action: 'submit->optimistic-take#submit' }
          ) do
            Button(
              type: :submit,
              variant: :primary,
              size: :md,
              class: 'inline-flex items-center gap-1 min-w-[80px]',
              data: { optimistic_take_target: 'button' }
            ) do
              plain t('person_medicines.card.take')
            end
          end
        else
          render_disabled_button_with_reason
        end
      end

      def render_disabled_button_with_reason
        reason = person_medicine.administration_blocked_reason
        label = reason == :out_of_stock ? t('person_medicines.card.out_of_stock') : t('person_medicines.card.take')
        Button(variant: :secondary, size: :md, disabled: true) { label }
      end

      def render_person_medicine_actions
        render_take_medicine_button if view_context.policy(person_medicine).take_medicine?
        render_delete_dialog if view_context.policy(person_medicine).destroy?
      end

      def render_delete_dialog
        return unless view_context.policy(person_medicine).destroy?

        AlertDialog do
          AlertDialogTrigger do
            Button(variant: :destructive_outline, size: :md) { t('person_medicines.card.remove') }
          end
          AlertDialogContent do
            AlertDialogHeader do
              AlertDialogTitle { t('person_medicines.card.remove_medicine') }
              AlertDialogDescription do
                plain t('person_medicines.card.remove_confirmation', medicine: person_medicine.medicine.name)
              end
            end
            AlertDialogFooter do
              AlertDialogCancel { t('dashboard.delete_confirmation.cancel') }
              form_with(
                url: person_person_medicine_path(person, person_medicine),
                method: :delete,
                class: 'inline'
              ) do
                Button(variant: :destructive, type: :submit) { t('person_medicines.card.remove') }
              end
            end
          end
        end
      end
    end
  end
end
