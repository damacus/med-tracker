# frozen_string_literal: true

module RubyUI
  class InputOtpSeparator < Base
    def view_template(&)
      div(**attrs) do
        if block_given?
          yield
        else
          icon
        end
      end
    end

    private

    def default_attrs
      {
        role: 'separator',
        class: 'text-on-surface-variant'
      }
    end

    def icon
      svg(
        xmlns: 'http://www.w3.org/2000/svg',
        viewbox: '0 0 24 24',
        fill: 'none',
        stroke: 'currentColor',
        stroke_width: '2',
        stroke_linecap: 'round',
        stroke_linejoin: 'round',
        class: 'h-4 w-4'
      ) { |s| s.path(d: 'M5 12h14') }
    end
  end
end
