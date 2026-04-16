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
      variant_classes = case @m3_variant
                        when :body_large then 'text-base leading-relaxed'
                        when :body_medium then 'text-sm leading-normal'
                        when :body_small then 'text-xs leading-tight'
                        when :label_large then 'text-sm font-medium tracking-wide'
                        when :label_medium then 'text-xs font-medium tracking-normal'
                        when :label_small then 'text-[11px] font-medium tracking-tight'
                        else 'text-sm'
                        end
      
      variant_classes
    end
  end
  end
end
