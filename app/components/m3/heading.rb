# frozen_string_literal: true

module Components
  module M3
  class Heading < RubyUI::Heading
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
      
      variant_classes = case @m3_variant
                        when :display_large then 'text-5xl lg:text-6xl font-normal tracking-tight'
                        when :display_medium then 'text-4xl lg:text-5xl font-normal'
                        when :display_small then 'text-3xl lg:text-4xl font-normal'
                        when :headline_large then 'text-3xl font-normal'
                        when :headline_medium then 'text-2xl font-normal'
                        when :headline_small then 'text-xl font-normal'
                        when :title_large then 'text-xl font-medium'
                        when :title_medium then 'text-base font-medium tracking-wide'
                        when :title_small then 'text-sm font-medium tracking-wide'
                        else 'text-base'
                        end
      
      "#{base_classes} #{variant_classes}"
    end
  end
  end
end
