# frozen_string_literal: true

module Components
  module M3
    class Text < RubyUI::Text
      def initialize(variant: :body_medium, **attrs)
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
        case @m3_variant
        when :body_large
          "text-base leading-relaxed"
        when :body_medium
          "text-sm leading-normal"
        when :body_small
          "text-xs leading-tight"
        when :label_large
          "text-sm font-medium tracking-wide"
        when :label_medium
          "text-xs font-medium tracking-normal"
        when :label_small
          "text-[11px] font-medium tracking-tight"
        else
          "text-sm"
        end
      end
    end
  end
end
