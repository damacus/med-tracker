# frozen_string_literal: true

module RubyUI
  class Progress < Base
    def initialize(value: 0, indicator_attrs: {}, **attrs)
      @value = value.to_f.clamp(0, 100)
      @indicator_attrs = indicator_attrs

      super(**attrs)
    end

    def view_template
      div(**attrs) do
        div(**indicator_attrs)
      end
    end

    private

    def default_attrs
      {
        role: 'progressbar',
        aria_valuenow: formatted_value,
        aria_valuemin: 0,
        aria_valuemax: 100,
        aria_valuetext: "#{formatted_value}%",
        class: 'relative h-2 overflow-hidden rounded-full bg-primary/20'
      }
    end

    def indicator_attrs
      attrs = mix(
        {
          class: 'h-full w-full flex-1 bg-primary',
          style: "transform: translateX(-#{formatted_offset}%);"
        },
        @indicator_attrs
      )
      attrs[:class] = TAILWIND_MERGER.merge(attrs[:class]) if attrs[:class]
      attrs
    end

    def formatted_value
      return @value.to_i.to_s if @value == @value.to_i

      @value.round(2).to_s
    end

    def formatted_offset
      offset = 100 - @value
      return offset.to_i.to_s if offset == offset.to_i

      offset.round(2).to_s
    end
  end
end
