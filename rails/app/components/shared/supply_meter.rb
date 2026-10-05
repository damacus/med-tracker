# frozen_string_literal: true

module Components
  module Shared
    class SupplyMeter < Components::Base
      attr_reader :percentage, :label, :fill_class, :track_class, :testid

      def initialize(percentage:, label:, fill_class:, **options)
        @percentage = percentage
        @label = label
        @fill_class = fill_class
        @track_class = options.fetch(:track_class, 'h-2 bg-surface-container shadow-inner')
        @testid = options.fetch(:testid, 'supply-meter')
        super()
      end

      def view_template
        render RubyUI::Progress.new(
          value: clamped_percentage,
          class: progress_classes,
          aria_label: label,
          data: { testid: testid },
          indicator_attrs: { class: indicator_background_class, data: { testid: "#{testid}-fill" } }
        )
      end

      private

      def clamped_percentage
        @clamped_percentage ||= (percentage || 0).to_f.clamp(0.0, 100.0)
      end

      def formatted_percentage
        return clamped_percentage.to_i.to_s if clamped_percentage == clamped_percentage.to_i

        clamped_percentage.round(2).to_s
      end

      def progress_classes
        "w-full #{track_class}"
      end

      def indicator_background_class
        fill_class.split.filter_map do |class_name|
          next unless class_name.start_with?('text-')

          "bg-#{class_name.delete_prefix('text-')}"
        end.join(' ')
      end
    end
  end
end
