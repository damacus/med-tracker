# frozen_string_literal: true

module Components
  module Shared
    class MedicationIcon < Components::Base
      ICONS_BY_UNIT = {
        'tablet' => Icons::Pill,
        'capsule' => Icons::Pill,
        'gummy' => Icons::Pill,
        'pill' => Icons::Pill,
        'ml' => Icons::Droplet,
        'drop' => Icons::Droplet,
        'spray' => Icons::Droplet,
        'iu' => Icons::Syringe
      }.freeze

      attr_reader :medication, :size, :attrs

      def initialize(medication: nil, unit: nil, size: 24, **attrs)
        @medication = medication
        @unit = unit
        @size = size
        @attrs = attrs
        super()
      end

      def view_template
        render icon_class.new(size: size, **attrs)
      end

      private

      def icon_class
        u = @unit || medication&.try(:dose_unit) || medication&.try(:unit)
        ICONS_BY_UNIT.fetch(u&.downcase, Icons::Medication)
      end
    end
  end
end
