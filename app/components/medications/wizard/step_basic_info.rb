# frozen_string_literal: true

module Components
  module Medications
    module Wizard
      class StepBasicInfo < Components::Base
        include FieldHelpers

        attr_reader :medication, :locations

        def initialize(medication:, locations:)
          @medication = medication
          @locations = locations
          super()
        end

        def view_template
          div(class: 'space-y-6') do
            div(class: 'space-y-1 mb-2') do
              Heading(level: 3, size: '5', class: 'font-bold tracking-tight text-slate-900') do
                'Medication Details'
              end
              Text(size: '2', class: 'text-slate-400') do
                'Tell us about this medication'
              end
            end

            render_location_field
            render_name_field
            render_category_field
            render_description_field
          end
        end
      end
    end
  end
end
