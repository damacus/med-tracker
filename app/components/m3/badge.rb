# frozen_string_literal: true

module Components
  module M3
    class Badge < RubyUI::Badge
      def initialize(variant: :filled, **attrs)
        @m3_variant = variant.to_sym

        base_variant = case @m3_variant
                       when :filled then :primary
                       when :tonal then :secondary
                       when :outlined then :outline
                       else @m3_variant
                       end

        super(variant: base_variant, **attrs)
      end

      private

      def primary_classes
        [
          %w[
            inline-flex
            items-center
            rounded-shape-full
            px-2.5
            py-0.5
            text-xs
            font-semibold
            bg-primary
            text-on-primary
          ].join(' ')
        ]
      end

      def secondary_classes
        [
          %w[
            inline-flex
            items-center
            rounded-shape-full
            px-2.5
            py-0.5
            text-xs
            font-semibold
            bg-secondary-container
            text-on-secondary-container
          ].join(' ')
        ]
      end

      def outline_classes
        [
          %w[
            inline-flex
            items-center
            rounded-shape-full
            border
            border-outline
            px-2.5
            py-0.5
            text-xs
            font-semibold
            text-primary
          ].join(' ')
        ]
      end

      def destructive_classes
        [
          %w[
            inline-flex
            items-center
            rounded-shape-full
            px-2.5
            py-0.5
            text-xs
            font-semibold
            bg-error
            text-on-error
          ].join(' ')
        ]
      end

      def default_classes
        case @m3_variant
        when :filled then primary_classes
        when :tonal then secondary_classes
        when :outlined then outline_classes
        when :destructive then destructive_classes
        else super
        end
      end
    end
  end
end
