# frozen_string_literal: true

module Components
  module Medications
    module Wizard
      class DosageFormFrame < Components::Base
        include Phlex::Rails::Helpers::TurboFrameTag

        attr_reader :medication, :dosage

        def initialize(medication:, dosage: nil)
          @medication = medication
          @dosage = dosage || medication.dosages.build
          super()
        end

        def view_template
          turbo_frame_tag 'dosage-form' do
            div(class: 'rounded-2xl border border-dashed border-border p-6 bg-surface-container-low') do
              render WizardDosageForm.new(
                dosage: dosage,
                medication: medication
              )
            end
          end
        end
      end
    end
  end
end
