# frozen_string_literal: true

module Components
  module Prescriptions
    # Renders a prescription card with medication details and take medicine form
    class Card < Components::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::ButtonTo

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
          render_medicine_icon
          CardTitle(class: 'text-xl') { prescription.medicine.name }
          dosage_text = "#{prescription.dosage.amount.to_i} #{prescription.dosage.unit}"
          CardDescription { "#{dosage_text} • #{prescription.frequency}" }
        end
      end

      def render_card_content
        CardContent(class: 'flex-grow space-y-4') do
          render_date_details
          render_notes if prescription.notes.present?
          render_countdown_notice if !prescription.can_take_now? && prescription.countdown_display
          render_takes_section
        end
      end

      def render_card_footer
        CardFooter(class: 'flex gap-2') do
          render_prescription_actions
        end
      end

      def render_medicine_icon
        div(class: 'w-10 h-10 rounded-xl flex items-center justify-center bg-violet-100 text-violet-700 mb-2') do
          render Icons::Pill.new(size: 20)
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

      def render_notes
        div(class: 'p-3 bg-amber-50 border border-amber-200 rounded-md') do
          Text(size: '2', class: 'text-amber-800') do
            span(class: 'font-semibold') { '📝 Notes: ' }
            plain prescription.notes
          end
        end
      end

      def render_countdown_notice
        div(class: 'p-3 bg-blue-50 border border-blue-200 rounded-md') do
          Text(size: '2', class: 'text-blue-800') do
            span(class: 'font-semibold') { '🕐 Next dose available in: ' }
            plain prescription.countdown_display
          end
        end
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
          render Icons::CheckCircle.new(size: 16, class: 'w-4 h-4 text-green-600')
          Text(as: 'span', weight: 'medium', class: 'text-slate-700') { take.taken_at.strftime('%l:%M %p').strip }
          Text(as: 'span', class: 'text-slate-500') { "#{take.amount_ml.to_i} ml" }
        end
      end

      def render_take_medicine_button
        if prescription.can_take_now?
          div(class: 'relative inline-block prescription__take-hover-card') do
            Button(variant: :primary, size: :sm, class: 'prescription__take-trigger') { '💊 Take' }
            render_take_medicine_form
          end
        else
          render_disabled_button_with_countdown
        end
      end

      def render_disabled_button_with_countdown
        Button(variant: :secondary, size: :sm, disabled: true) { '💊 Take' }
      end

      def render_take_medicine_form
        form_with(
          url: take_medicine_person_prescription_path(person, prescription),
          method: :post,
          class: 'prescription__take-form'
        ) do |f|
          div(class: 'prescription__take-form-group') do
            render f.label(:amount_ml, 'Amount (ml)', class: 'text-sm font-medium text-slate-700')
            render f.select(
              :amount_ml,
              dosage_options,
              { selected: prescription.dosage.amount },
              class: 'w-full px-3 py-2 border border-slate-300 rounded-md text-sm ' \
                     'focus:outline-none focus:ring-2 focus:ring-primary'
            )
            Button(type: :submit, variant: :primary, size: :md, class: 'w-full mt-3') { 'Take Now' }
          end
        end
      end

      def dosage_options
        prescription.medicine.dosages.map do |dosage|
          ["#{dosage.amount.to_i} #{dosage.unit} - #{dosage.description}", dosage.amount]
        end
      end

      def render_prescription_actions
        render_take_medicine_button
        return unless view_context.current_user&.administrator?

        Link(href: edit_person_prescription_path(person, prescription), variant: :outline) { 'Edit' }
        render_delete_dialog
      end

      def render_delete_dialog
        AlertDialog do
          AlertDialogTrigger do
            Button(variant: :destructive, size: :md) { 'Delete' }
          end
          AlertDialogContent do
            AlertDialogHeader do
              AlertDialogTitle { 'Delete Prescription' }
              AlertDialogDescription do
                plain "Are you sure you want to delete the #{prescription.medicine.name} prescription? "
                plain 'This action cannot be undone.'
              end
            end
            AlertDialogFooter do
              Button(
                type: :button,
                variant: :outline,
                data: { action: 'click->ruby-ui--alert-dialog#close' }
              ) { 'Cancel' }
              Link(
                href: person_prescription_path(person, prescription),
                variant: :destructive,
                data: {
                  turbo_method: :delete,
                  turbo_confirm: "Are you sure you want to delete this prescription? This action cannot be undone.",
                  action: 'click->ruby-ui--alert-dialog#close'
                }
              ) { 'Delete' }
            end
          end
        end
      end
    end
  end
end
