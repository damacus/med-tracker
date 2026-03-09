# frozen_string_literal: true

module Components
  module Medications
    module Wizard
      class ModalWrapper < Components::Base
        include Phlex::Rails::Helpers::TurboFrameTag

        attr_reader :medication, :locations, :variant

        def initialize(medication:, locations:)
          @medication = medication
          @locations = locations
          @variant = 'modal'
          super()
        end

        def view_template
          turbo_frame_tag 'modal' do
            Dialog(open: true) do
              DialogContent(size: :xl) do
                DialogHeader do
                  DialogTitle { t('medications.form.new_title') }
                  DialogDescription { t('medications.form.new_subtitle') }
                end
                DialogMiddle(class: 'max-h-[70vh] overflow-y-auto px-1') do
                  render StepContent.new(
                    medication: medication,
                    locations: locations,
                    variant: variant
                  )
                end
              end
            end
          end
        end
      end
    end
  end
end
