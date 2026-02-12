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
          Text(size: '2', weight: 'medium', class: 'uppercase tracking-wide text-slate-500 mb-2') do
            'Add Medicine'
          end
          Heading(level: 1) { "Add Medicine for #{person.name}" }
        end
      end

      def render_form
        form_with(
          model: person_medicine,
          url: person_person_medicines_path(person),
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
            ul(class: 'my-2 ml-6 list-disc [&>li]:mt-1') do
              person_medicine.errors.full_messages.each do |message|
                li { message }
              end
            end
          end
        end
      end

      def render_form_fields(_form)
        render FormFields.new(person_medicine: person_medicine, medicines: medicines)
      end

      def render_actions
        div(class: 'flex justify-end gap-3 pt-4') do
          Link(href: person_path(person), variant: :outline) { 'Cancel' }
          Button(type: :submit, variant: :primary) { 'Add Medicine' }
        end
      end
    end
  end
end
