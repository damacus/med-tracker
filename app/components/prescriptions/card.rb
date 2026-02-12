# frozen_string_literal: true

module Components
  module Prescriptions
    # Renders a prescription card with medication details and take medicine form
    class Card < Components::Base
      attr_reader :prescription, :person

      def initialize(prescription:, person:)
        @prescription = prescription
        @person = person
        super()
      end

      def view_template
        RubyUI::Card(id: "prescription_#{prescription.id}", class: 'h-full flex flex-col') do
          render_card_header
          render_card_content
          render_card_footer
        end
      end

      private

      def render_card_header
        CardHeader do
          div(class: 'flex justify-between items-start w-full') do
            div do
              render Components::Shared::MedicineIcon.new
              CardTitle(class: 'text-xl') { prescription.medicine.name }
              dosage_text = "#{prescription.dosage.amount.to_i} #{prescription.dosage.unit}"
              CardDescription { "#{dosage_text} • #{prescription.frequency}" }
            end
            render Components::Shared::StockBadge.new(medicine: prescription.medicine)
          end
        end
      end

      def render_card_content
        CardContent(class: 'flex-grow space-y-4') do
          render_date_details
          render Components::Shared::NotesSection.new(notes: prescription.notes)
          render_countdown_notice
          render_takes_section
        end
      end

      def render_card_footer
        CardFooter(class: 'flex gap-2') do
          render_prescription_actions
        end
      end

      def render_date_details
        div(class: 'space-y-1 text-sm') do
          div(class: 'flex items-center gap-2') do
            Text(as: 'span', class: 'text-slate-500') { '📅 Started:' }
            Text(as: 'span', weight: 'medium', class: 'text-slate-700') do
              prescription.start_date.strftime('%B %d, %Y')
            end
          end

          if prescription.end_date
            div(class: 'flex items-center gap-2') do
              Text(as: 'span', class: 'text-slate-500') { '🏁 Ends:' }
              Text(as: 'span', weight: 'medium', class: 'text-slate-700') do
                prescription.end_date.strftime('%B %d, %Y')
              end
            end
          end
        end
      end

      def render_countdown_notice
        return if prescription.can_take_now? || prescription.countdown_display.blank?

        render Components::Shared::CountdownNotice.new(countdown_display: prescription.countdown_display)
      end

      def render_takes_section
        div(class: 'space-y-3') do
          Heading(level: 4, size: '2', class: 'font-semibold text-slate-700') { "Today's Doses" }
          render_todays_takes
        end
      end

      def render_todays_takes
        todays_takes = prescription.medication_takes.where(taken_at: Time.current.beginning_of_day..)

        if todays_takes.any?
          div(class: 'space-y-2') do
            todays_takes.order(taken_at: :desc).each do |take|
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
          Text(as: 'span', class: 'text-slate-500') { "#{take.amount_ml.to_i} #{prescription.dosage.unit}" }
        end
      end

      def render_prescription_actions
        render Components::Shared::TakeMedicineButton.new(
          takeable: prescription,
          take_url: take_medicine_person_prescription_path(person, prescription)
        )
        return unless view_context.current_user&.administrator?

        Link(href: edit_person_prescription_path(person, prescription), variant: :outline) { 'Edit' }
        medicine_name = prescription.medicine.name
        render Components::Shared::DeleteConfirmDialog.new(
          title: 'Delete Prescription',
          description: "Are you sure you want to delete the #{medicine_name} prescription? " \
                       'This action cannot be undone.',
          delete_url: person_prescription_path(person, prescription)
        )
      end
    end
  end
end
