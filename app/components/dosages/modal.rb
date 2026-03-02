# frozen_string_literal: true

module Components
  module Dosages
    # Modal wrapper for the dosage create/edit form
    class Modal < Components::Base
      include Phlex::Rails::Helpers::TurboFrameTag
      include RubyUI

      attr_reader :dosage, :medication

      def initialize(dosage:, medication:)
        @dosage = dosage
        @medication = medication
        super()
      end

      def view_template
        turbo_frame_tag 'modal' do
          Dialog(open: true) do
            DialogContent(size: :lg) do
              DialogHeader do
                DialogTitle { dosage.new_record? ? t('dosages.new.title') : t('dosages.edit.title') }
                DialogDescription { medication.name }
              end
              DialogMiddle do
                render Form.new(dosage: dosage, medication: medication)
              end
            end
          end
        end
      end
    end
  end
end
