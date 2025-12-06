# frozen_string_literal: true

module Components
  module PersonMedicines
    # Form view for adding a person medicine (OTC/supplement)
    class FormView < Components::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::OptionsFromCollectionForSelect
      include Phlex::Rails::Helpers::Pluralize

      attr_reader :person_medicine, :person, :medicines

      def initialize(person_medicine:, person:, medicines:)
        @person_medicine = person_medicine
        @person = person
        @medicines = medicines
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-8 max-w-2xl') do
          render_header
          render_form
        end
      end

      private

      def render_header
        div(class: 'mb-8') do
          p(class: 'text-sm font-medium uppercase tracking-wide text-slate-500 mb-2') do
            'Add Medicine'
          end
          h1(class: 'text-4xl font-bold text-slate-900') do
            "Add Medicine for #{person.name}"
          end
        end
      end

      def render_form
        form_with(
          model: person_medicine,
          url: helpers.person_person_medicines_path(person),
          class: 'space-y-6'
        ) do |form|
          render_errors if person_medicine.errors.any?
          render_form_fields(form)
          render_actions
        end
      end

      def render_errors
        Alert(variant: :destructive, class: 'mb-6') do
          AlertTitle do
            "#{pluralize(person_medicine.errors.count, 'error')} prohibited this medicine from being saved:"
          end
          AlertDescription do
            ul(class: 'list-disc list-inside space-y-1') do
              person_medicine.errors.full_messages.each do |message|
                li { message }
              end
            end
          end
        end
      end

      def render_form_fields(_form)
        div(class: 'space-y-6') do
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
          Link(href: helpers.person_path(person), variant: :outline) { 'Cancel' }
          Button(type: :submit, variant: :primary) { 'Add Medicine' }
        end
      end

      def select_classes
        'flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ' \
          'ring-offset-background focus-visible:outline-none focus-visible:ring-2 ' \
          'focus-visible:ring-ring focus-visible:ring-offset-2'
      end
    end
  end
end
