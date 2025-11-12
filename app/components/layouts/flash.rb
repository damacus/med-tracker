# frozen_string_literal: true

module Components
  module Layouts
    class Flash < Components::Base
      def initialize(notice: nil, alert: nil)
        @notice = notice
        @alert = alert
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-4') do
          render_notice if @notice
          render_alert if @alert
        end
      end

      private

      def render_notice
        Alert(variant: :success) do
          check_icon
          AlertTitle { 'Success' }
          AlertDescription { @notice }
        end
      end

      def render_alert
        Alert(variant: :destructive) do
          alert_icon
          AlertTitle { 'Error' }
          AlertDescription { @alert }
        end
      end

      def check_icon
        svg(
          xmlns: 'http://www.w3.org/2000/svg',
          width: '16',
          height: '16',
          viewBox: '0 0 24 24',
          fill: 'none',
          stroke: 'currentColor',
          stroke_width: '2',
          stroke_linecap: 'round',
          stroke_linejoin: 'round',
          class: 'lucide lucide-check'
        ) do |s|
          s.path(d: 'M20 6 9 17l-5-5')
        end
      end

      def alert_icon
        svg(
          xmlns: 'http://www.w3.org/2000/svg',
          width: '16',
          height: '16',
          viewBox: '0 0 24 24',
          fill: 'none',
          stroke: 'currentColor',
          stroke_width: '2',
          stroke_linecap: 'round',
          stroke_linejoin: 'round',
          class: 'lucide lucide-alert-circle'
        ) do |s|
          s.circle(cx: '12', cy: '12', r: '10')
          s.path(d: 'M12 8v4')
          s.path(d: 'M12 16h.01')
        end
      end
    end
  end
end
