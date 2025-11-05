# frozen_string_literal: true

module Components
  module PersonMedicines
    # Renders a person medicine card with take medicine functionality
    class Card < Components::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::ButtonTo

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
          parts << "#{person_medicine.medicine.dosage_amount} #{person_medicine.medicine.dosage_unit}"
        end
        parts.join(' • ')
      end

      def render_medicine_icon
        div(class: 'w-10 h-10 rounded-xl flex items-center justify-center bg-green-100 text-green-700 mb-2') do
          svg(
            xmlns: 'http://www.w3.org/2000/svg',
            width: '20',
            height: '20',
            viewBox: '0 0 24 24',
            fill: 'none',
            stroke: 'currentColor',
            stroke_width: '2',
            stroke_linecap: 'round',
            stroke_linejoin: 'round'
          ) do |s|
            s.path(d: 'M10.5 20.5 10 21a2 2 0 0 1-2.828 0L4.343 18.172a2 2 0 0 1 0-2.828l.5-.5')
            s.path(d: 'm7 17-5-5')
            s.path(d: 'M13.5 3.5 14 3a2 2 0 0 1 2.828 0l2.829 2.828a2 2 0 0 1 0 2.829l-.5.5')
            s.path(d: 'm17 7 5 5')
            s.circle(cx: '12', cy: '12', r: '2')
          end
        end
      end

      def render_notes
        div(class: 'p-3 bg-blue-50 border border-blue-200 rounded-md') do
          p(class: 'text-sm text-blue-800') do
            span(class: 'font-semibold') { '📝 Notes: ' }
            plain person_medicine.notes
          end
        end
      end

      def render_timing_restrictions
        div(class: 'p-3 bg-amber-50 border border-amber-200 rounded-md') do
          p(class: 'text-sm text-amber-800 font-semibold mb-1') { '⏱️ Timing Restrictions:' }
          ul(class: 'text-sm text-amber-800 list-disc list-inside') do
            if person_medicine.max_daily_doses.present?
              li { "Maximum #{person_medicine.max_daily_doses} dose(s) per day" }
            end
            if person_medicine.min_hours_between_doses.present?
              li { "Wait at least #{person_medicine.min_hours_between_doses} hours between doses" }
            end
          end
        end
      end

      def render_takes_section
        div(class: 'space-y-3') do
          div(class: 'flex items-center justify-between') do
            h4(class: 'text-sm font-semibold text-slate-700') { "Today's Doses" }
            render_take_medicine_button
          end
          render_todays_takes
        end
      end

      def render_todays_takes
        todays_takes = person_medicine.medication_takes.where(taken_at: Time.current.beginning_of_day..)

        if todays_takes.any?
          div(class: 'space-y-2') do
            todays_takes.order(taken_at: :desc).each do |take|
              render_take_item(take)
            end
          end
        else
          p(class: 'text-sm text-slate-500 italic') { 'No doses taken today' }
        end
      end

      def render_take_item(take)
        div(class: 'flex items-center gap-2 text-sm') do
          svg(
            class: 'w-4 h-4 text-green-600',
            xmlns: 'http://www.w3.org/2000/svg',
            viewBox: '0 0 20 20',
            fill: 'currentColor'
          ) do |s|
            s.path(
              fill_rule: 'evenodd',
              d: 'M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 ' \
                 '12.586l7.293-7.293a1 1 0 011.414 0z',
              clip_rule: 'evenodd'
            )
          end
          span(class: 'text-slate-700 font-medium') { take.taken_at.strftime('%l:%M %p').strip }
          span(class: 'text-slate-500') { "#{take.amount_ml} ml" } if take.amount_ml.present?
        end
      end

      def render_take_medicine_button
        return unless view_context.policy(person_medicine).take_medicine?

        can_take = person_medicine.can_take_now?

        Button(
          variant: :primary,
          size: :sm,
          disabled: !can_take,
          data: { action: 'click->person-medicine#take' }
        ) do
          plain '💊 Take'
        end

        return if can_take

        render_disabled_reason
      end

      def render_disabled_reason
        next_time = person_medicine.next_available_time
        return unless next_time

        p(class: 'text-xs text-amber-600 mt-1 font-medium') do
          plain view_context.time_until_available(next_time)
        end
      end

      def render_person_medicine_actions
        can_take = person_medicine.can_take_now?

        if view_context.policy(person_medicine).take_medicine?
          div(class: 'flex flex-col gap-1') do
            button_to(
              take_medicine_person_person_medicine_path(person, person_medicine),
              method: :post,
              disabled: !can_take,
              class: 'inline-flex items-center justify-center rounded-md text-sm font-medium px-4 py-2 ' \
                     'bg-primary text-white shadow-sm hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed'
            ) { '💊 Take Now' }

            if !can_take && person_medicine.next_available_time
              p(class: 'text-xs text-amber-600 font-medium') do
                plain view_context.time_until_available(person_medicine.next_available_time)
              end
            end
          end
        end

        render_delete_dialog
      end

      def render_delete_dialog
        return unless view_context.policy(person_medicine).destroy?

        AlertDialog do
          AlertDialogTrigger do
            Button(variant: :destructive, size: :md) { 'Remove' }
          end
          AlertDialogContent do
            AlertDialogHeader do
              AlertDialogTitle { 'Remove Medicine' }
              AlertDialogDescription do
                plain "Are you sure you want to remove #{person_medicine.medicine.name}? "
                plain 'This action cannot be undone.'
              end
            end
            AlertDialogFooter do
              button(
                type: 'button',
                data: { action: 'click->ruby-ui--alert-dialog#close' },
                class: 'inline-flex items-center justify-center rounded-md text-sm font-medium px-4 py-2 ' \
                       'border border-input bg-background hover:bg-accent hover:text-accent-foreground'
              ) { 'Cancel' }
              button_to(
                person_person_medicine_path(person, person_medicine),
                method: :delete,
                class: 'inline-flex items-center justify-center rounded-md text-sm font-medium px-4 py-2 ' \
                       'bg-destructive text-white shadow-sm hover:bg-destructive/90'
              ) { 'Remove' }
            end
          end
        end
      end
    end
  end
end
