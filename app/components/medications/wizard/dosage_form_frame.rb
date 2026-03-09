# frozen_string_literal: true

module Components
  module Medications
    module Wizard
      class DosageFormFrame < Components::Base
        include Phlex::Rails::Helpers::TurboFrameTag

        attr_reader :medication

        def initialize(medication:)
          @medication = medication
          super()
        end

        def view_template
          turbo_frame_tag 'dosage-form' do
            div(class: 'rounded-2xl border border-dashed border-slate-200 p-6 bg-slate-50/50') do
              render WizardDosageForm.new(
                dosage: medication.dosages.build,
                medication: medication
              )
            end
          end
        end
      end
    end
  end
end
