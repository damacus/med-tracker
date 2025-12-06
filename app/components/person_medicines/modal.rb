# frozen_string_literal: true

module Components
  module PersonMedicines
    # Modal component for person medicine form
    class Modal < Components::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::TurboFrameTag

      attr_reader :person_medicine, :person, :medicines, :title

      def initialize(person_medicine:, person:, medicines:, title: nil)
        @person_medicine = person_medicine
        @person = person
        @medicines = medicines
        @title = title || "Add Medicine for #{person.name}"
        super()
      end

      def view_template
        turbo_frame_tag 'person_medicine_modal' do
          div(class: 'fixed inset-0 z-50 flex items-center justify-center') do
            div(class: 'fixed inset-0 bg-background/80 backdrop-blur-sm')
            div(class: 'relative z-50 w-full max-w-lg border bg-background p-6 shadow-lg rounded-lg') do
              render_header
              render_form
              render_close_button
            end
          end
        end
      end

      private

      def render_header
        div(class: 'mb-4') do
          h2(class: 'text-lg font-semibold') { title }
          p(class: 'text-sm text-muted-foreground') { 'Add a vitamin, supplement, or over-the-counter medicine' }
        end
      end

      def render_close_button
        a(
          href: helpers.person_path(person),
          class: 'absolute right-4 top-4 rounded-sm opacity-70 hover:opacity-100',
          data: { turbo_frame: 'person_medicine_modal' }
        ) do
          plain '×'
        end
      end

      def render_form
        form_with(
          model: person_medicine,
          url: helpers.person_person_medicines_path(person),
          class: 'space-y-6'
        ) do |_form|
          render_form_fields
          render_actions
        end
      end

      def render_form_fields
        div(class: 'space-y-4') do
          render_medicine_field
          render_notes_field
          render_timing_fields
        end
      end

      def render_medicine_field
        FormField do
          FormFieldLabel(for: 'person_medicine_medicine_id') { 'Medicine' }
          select(
            name: 'person_medicine[medicine_id]',
            id: 'person_medicine_medicine_id',
            required: true,
            class: select_classes
          ) do
            option(value: '', disabled: true, selected: person_medicine.medicine_id.blank?) { 'Select a medicine' }
            medicines.each do |medicine|
              option(value: medicine.id, selected: person_medicine.medicine_id == medicine.id) { medicine.name }
            end
          end
          FormFieldHint { 'Select a medicine from the list' }
        end
      end

      def render_notes_field
        FormField do
          FormFieldLabel(for: 'person_medicine_notes') { 'Notes' }
          Textarea(
            name: 'person_medicine[notes]',
            id: 'person_medicine_notes',
            placeholder: 'Add any special instructions or notes',
            rows: 3
          ) { person_medicine.notes }
          FormFieldHint { 'Add any special instructions or notes' }
        end
      end

      def render_timing_fields
        div(class: 'grid grid-cols-2 gap-4') do
          render_max_daily_doses_field
          render_min_hours_field
        end
      end

      def render_max_daily_doses_field
        FormField do
          FormFieldLabel(for: 'person_medicine_max_daily_doses') { 'Max daily doses' }
          Input(
            type: :number,
            name: 'person_medicine[max_daily_doses]',
            id: 'person_medicine_max_daily_doses',
            value: person_medicine.max_daily_doses,
            min: 1,
            placeholder: 'Optional'
          )
          FormFieldHint { 'Maximum doses per day' }
        end
      end

      def render_min_hours_field
        FormField do
          FormFieldLabel(for: 'person_medicine_min_hours_between_doses') { 'Min hours between doses' }
          Input(
            type: :number,
            name: 'person_medicine[min_hours_between_doses]',
            id: 'person_medicine_min_hours_between_doses',
            value: person_medicine.min_hours_between_doses,
            min: 1,
            step: 0.5,
            placeholder: 'Optional'
          )
          FormFieldHint { 'Minimum time between doses' }
        end
      end

      def render_actions
        div(class: 'flex justify-end gap-3 pt-4') do
          a(
            href: helpers.person_path(person),
            class: button_outline_classes,
            data: { turbo_frame: 'person_medicine_modal' }
          ) { 'Cancel' }
          Button(type: :submit, variant: :primary) { 'Add Medicine' }
        end
      end

      def button_outline_classes
        'inline-flex items-center justify-center whitespace-nowrap rounded-md text-sm font-medium ' \
          'ring-offset-background transition-colors focus-visible:outline-none focus-visible:ring-2 ' \
          'focus-visible:ring-ring focus-visible:ring-offset-2 border border-input bg-background ' \
          'hover:bg-accent hover:text-accent-foreground h-10 px-4 py-2'
      end

      def select_classes
        'flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ' \
          'ring-offset-background focus-visible:outline-none focus-visible:ring-2 ' \
          'focus-visible:ring-ring focus-visible:ring-offset-2'
      end
    end
  end
end
