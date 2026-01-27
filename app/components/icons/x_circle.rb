# frozen_string_literal: true

module Components
  module Icons
    class XCircle < Base
      def initialize(size: 16, **attrs)
        super
        @attrs[:viewBox] = '0 0 20 20'
        @attrs[:fill] = 'currentColor'
        @attrs[:stroke] = nil
        @attrs[:stroke_width] = nil
        @attrs[:stroke_linecap] = nil
        @attrs[:stroke_linejoin] = nil
      end

      def view_template
        svg(**merged_attrs.compact) do |s|
          s.path(
            fill_rule: 'evenodd',
            d: 'M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 ' \
               '1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 ' \
               '1 0 00-1.414-1.414L10 8.586 8.707 7.293z',
            clip_rule: 'evenodd'
          )
        end
      end
    end
  end
end
