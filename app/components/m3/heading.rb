# frozen_string_literal: true

module Components
  module M3
    class Heading < RubyUI::Heading
      VARIANT_CLASSES = {
        display_large: 'text-5xl lg:text-6xl font-normal tracking-tight',
        display_medium: 'text-4xl lg:text-5xl font-normal',
        display_small: 'text-3xl lg:text-4xl font-normal',
        headline_large: 'text-3xl font-normal',
        headline_medium: 'text-2xl font-normal',
        headline_small: 'text-xl font-normal',
        title_large: 'text-xl font-medium',
        title_medium: 'text-base font-medium tracking-wide',
        title_small: 'text-sm font-medium tracking-wide'
      }.freeze

      def initialize(variant: :headline_small, **attrs)
        @m3_variant = variant.to_sym
        super(**attrs)
      end

      private

      def default_attrs
        {
          class: class_names
        }
      end

      def class_names
        base_classes = 'scroll-m-20'
        variant_classes = VARIANT_CLASSES.fetch(@m3_variant, 'text-base')

        "#{base_classes} #{variant_classes}"
      end
    end
  end
end
