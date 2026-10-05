# frozen_string_literal: true

module Components
  module Icons
    class Base < ::RubyUI::Base
      DEFAULT_SIZE = 16

      def initialize(size: DEFAULT_SIZE, **attrs)
        @size = size
        super(**attrs)
      end

      private

      attr_reader :size

      def default_attrs
        {
          xmlns: 'http://www.w3.org/2000/svg',
          width: size.to_s,
          height: size.to_s,
          viewBox: '0 0 24 24',
          fill: 'none',
          stroke: 'currentColor',
          stroke_width: '2',
          stroke_linecap: 'round',
          stroke_linejoin: 'round',
          class: "lucide lucide-#{self.class.name.demodulize.underscore.dasherize}"
        }
      end
    end
  end
end
