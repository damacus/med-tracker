# frozen_string_literal: true

module Components
  module Icons
    class Base < Phlex::HTML
      DEFAULT_SIZE = 16

      def initialize(size: DEFAULT_SIZE, **attrs)
        @size = size
        @attrs = attrs
        super()
      end

      private

      attr_reader :size, :attrs

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
          stroke_linejoin: 'round'
        }
      end

      def merged_attrs
        default_attrs.merge(attrs)
      end
    end
  end
end
