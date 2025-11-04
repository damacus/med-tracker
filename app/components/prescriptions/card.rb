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
          CardDescription { "#{prescription.dosage.amount} #{prescription.dosage.unit} • #{prescription.frequency}" }
        end
      end

      def render_card_content
        CardContent(class: 'flex-grow space-y-4') do
          render_date_details
          render_notes if prescription.notes.present?
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

      def render_date_details
        div(class: 'space-y-1 text-sm') do
          div(class: 'flex items-center gap-2') do
            span(class: 'text-slate-500') { '📅 Started:' }
            span(class: 'text-slate-700 font-medium') { prescription.start_date.strftime('%B %d, %Y') }
          end

          if prescription.end_date
            div(class: 'flex items-center gap-2') do
              span(class: 'text-slate-500') { '🏁 Ends:' }
              span(class: 'text-slate-700 font-medium') { prescription.end_date.strftime('%B %d, %Y') }
            end
          end
        end
      end

      def render_notes
        div(class: 'p-3 bg-amber-50 border border-amber-200 rounded-md') do
          p(class: 'text-sm text-amber-800') do
            span(class: 'font-semibold') { '📝 Notes: ' }
            plain prescription.notes
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
        todays_takes = prescription.take_medicines.where(taken_at: Time.current.beginning_of_day..)

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
            xmlns: 'http://www.w3.org/2000/svg',
            viewBox: '0 0 20 20',
            fill: 'currentColor',
            class: 'w-4 h-4 text-green-600',
            width: '16',
            height: '16'
          ) do |s|
            s.path(
              fill_rule: 'evenodd',
              d: 'M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 ' \
                 '12.586l7.293-7.293a1 1 0 011.414 0z',
              clip_rule: 'evenodd'
            )
          end
          span(class: 'text-slate-700 font-medium') { take.taken_at.strftime('%l:%M %p').strip }
          span(class: 'text-slate-500') { "#{take.amount_ml} ml" }
        end
      end

      def render_take_medicine_button
        div(class: 'relative inline-block prescription__take-hover-card') do
          Button(variant: :primary, size: :sm, class: 'prescription__take-trigger') { '💊 Take' }
          render_take_medicine_form
        end
      end

      def render_take_medicine_form
        form_with(
          url: take_medicine_person_prescription_path(person, prescription),
          method: :post,
          class: 'prescription__take-form'
        ) do |f|
          div(class: 'prescription__take-form-group') do
            render f.label(:amount_ml, 'Amount (ml)', class: 'text-sm font-medium text-slate-700')
            render f.number_field(
              :amount_ml,
              value: prescription.dosage.amount,
              step: 0.5,
              min: 0,
              class: 'w-full px-3 py-2 border border-slate-300 rounded-md text-sm ' \
                     'focus:outline-none focus:ring-2 focus:ring-primary'
            )
            Button(type: :submit, variant: :primary, size: :md, class: 'w-full') { 'Take Now' }
          end
        end
      end

      def render_prescription_actions
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
              button_to(
                person_prescription_path(person, prescription),
                method: :delete,
                class: 'inline-flex items-center justify-center rounded-md text-sm font-medium px-4 py-2 ' \
                       'bg-destructive text-white shadow-sm hover:bg-destructive/90'
              ) { 'Delete' }
            end
          end
        end
      end
    end
  end
end
