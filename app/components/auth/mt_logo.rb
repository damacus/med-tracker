# frozen_string_literal: true

module Components
  module Auth
    class MtLogo < Components::Base
      PATHS = [
        {
          d: 'M8 40V18C8 11.4 13.4 6 20 6C26.6 6 32 11.4 32 18V40',
          stroke: '#14A99A', stroke_width: '6', stroke_linecap: 'round'
        },
        {
          d: 'M32 40V18C32 11.4 37.4 6 44 6C50.6 6 56 11.4 56 18V40',
          stroke: '#14A99A', stroke_width: '6', stroke_linecap: 'round'
        },
        { d: 'M64 6V40', stroke: '#14A99A', stroke_width: '6', stroke_linecap: 'round' },
        { d: 'M58 18H80', stroke: '#14A99A', stroke_width: '6', stroke_linecap: 'round' },
        { d: 'M78 4V14', stroke: '#14A99A', stroke_width: '5', stroke_linecap: 'round' }
      ].freeze

      def initialize(label:)
        @label = label
        super()
      end

      def view_template
        svg(width: '96', height: '48', viewbox: '0 0 96 48', fill: 'none', xmlns: 'http://www.w3.org/2000/svg',
            data_login_logo: 'mt', aria_label: label, role: 'img',
            class: 'h-10 w-20 md:h-12 md:w-24') do |s|
          PATHS.each { |attrs| s.path(**attrs) }
        end
      end

      private

      attr_reader :label
    end
  end
end
