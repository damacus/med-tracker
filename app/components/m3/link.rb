# frozen_string_literal: true

module Components
  module M3
    class Link < RubyUI::Link
      BASE_CLASSES = [
        'inline-flex items-center justify-center rounded-shape-full font-medium transition-all state-layer',
        'disabled:pointer-events-none disabled:opacity-38',
        'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-secondary',
        'aria-disabled:pointer-events-none aria-disabled:opacity-38 aria-disabled:cursor-not-allowed'
      ].freeze

      def initialize(variant: :filled, **attrs)
        @m3_variant = variant.to_sym

        base_variant = case @m3_variant
                       when :filled then :primary
                       when :tonal then :secondary
                       when :text then :ghost
                       when :outlined then :outline
                       else @m3_variant
                       end

        super(variant: base_variant, **attrs)
      end

      def default_attrs
        { class: default_classes }
      end

      private

      def primary_classes
        [
          BASE_CLASSES,
          size_classes,
          'bg-primary text-on-primary no-underline'
        ]
      end

      def secondary_classes
        [
          BASE_CLASSES,
          size_classes,
          'bg-secondary-container text-on-secondary-container no-underline'
        ]
      end

      def outline_classes
        [
          BASE_CLASSES,
          size_classes,
          'border border-outline bg-transparent text-primary no-underline'
        ]
      end

      def ghost_classes
        [
          BASE_CLASSES,
          size_classes,
          'bg-transparent text-primary no-underline'
        ]
      end

      def default_classes
        case @m3_variant
        when :filled then primary_classes
        when :tonal then secondary_classes
        when :outlined then outline_classes
        when :text then ghost_classes
        else super
        end
      end
    end
  end
end
