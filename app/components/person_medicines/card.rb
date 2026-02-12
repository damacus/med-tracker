# frozen_string_literal: true

module Components
  module PersonMedicines
    # Renders a person medicine card with take medicine functionality
    class Card < Components::Base
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
          render Components::Shared::MedicineIcon.new
          CardTitle(class: 'text-xl') { person_medicine.medicine.name }
          CardDescription { medicine_description }
        end
      end

      def render_card_content
        CardContent(class: 'flex-grow space-y-4') do
          render Components::Shared::NotesSection.new(notes: person_medicine.notes)
          render_timing_restrictions if person_medicine.timing_restrictions?
          render_countdown_notice
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

      def render_timing_restrictions
        div(class: 'p-3 bg-amber-50 border border-amber-200 rounded-md') do
          Text(size: '2', weight: 'semibold', class: 'text-amber-800 mb-1') { '⏱️ Timing Restrictions:' }
          ul(class: 'my-1 ml-4 text-sm text-amber-800 list-disc [&>li]:mt-0.5') do
            if person_medicine.max_daily_doses.present?
              li { "Maximum #{person_medicine.max_daily_doses} dose(s) per day" }
            end
            if person_medicine.min_hours_between_doses.present?
              li { "Wait at least #{person_medicine.min_hours_between_doses} hours between doses" }
            end
          end
        end
      end

      def render_countdown_notice
        return if person_medicine.can_take_now? || person_medicine.countdown_display.blank?

        render Components::Shared::CountdownNotice.new(countdown_display: person_medicine.countdown_display)
      end

      def render_takes_section
        todays_takes = person_medicine.medication_takes
                                      .where(taken_at: Time.current.beginning_of_day..)
                                      .order(taken_at: :desc)
                                      .load

        div(class: 'space-y-3') do
          div(class: 'flex items-center justify-between') do
            Heading(level: 4, size: '2', class: 'font-semibold text-slate-700') { "Today's Doses" }
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
          Text(size: '2', weight: 'muted', class: 'italic') { 'No doses taken today' }
        end
      end

      def render_take_item(take)
        div(class: 'flex items-center gap-2 text-sm') do
          render Icons::CheckCircle.new(size: 16, class: 'w-4 h-4 text-success')
          Text(as: 'span', weight: 'medium', class: 'text-slate-700') { take.taken_at.strftime('%l:%M %p').strip }
          Text(as: 'span', class: 'text-slate-500') { "#{take.amount_ml.to_i} ml" } if take.amount_ml.present?
        end
      end

      def render_person_medicine_actions
        if view_context.policy(person_medicine).take_medicine?
          render Components::Shared::TakeMedicineButton.new(
            takeable: person_medicine,
            take_url: take_medicine_person_person_medicine_path(person, person_medicine)
          )
        end
        return unless view_context.policy(person_medicine).destroy?

        medicine_name = person_medicine.medicine.name
        render Components::Shared::DeleteConfirmDialog.new(
          title: 'Remove Medicine',
          description: "Are you sure you want to remove #{medicine_name}? " \
                       'This action cannot be undone.',
          delete_url: person_person_medicine_path(person, person_medicine),
          trigger_label: 'Remove',
          confirm_label: 'Remove'
        )
      end
    end
  end
end
