# frozen_string_literal: true

module Components
  module Auth
    class MtLogo < Components::Base
      def initialize(label:)
        @label = label
        super()
      end

      def view_template
        svg(
          width: "192",
          height: "96",
          viewBox: "0 0 192 96",
          fill: "none",
          xmlns: "http://www.w3.org/2000/svg",
          data_login_logo: "mt",
          aria_label: label,
          role: "img",
          class: "h-10 w-20 md:h-12 md:w-24"
        ) do |s|
          s.defs do
            s
              .linearGradient(
                id: "med-tracker-logo-teal",
                x1: "18",
                y1: "20",
                x2: "126",
                y2: "84",
                gradientUnits: "userSpaceOnUse"
              ) do |gradient|
                gradient.stop(offset: "0%", stop_color: "#78D0C4")
                gradient.stop(offset: "100%", stop_color: "#0C978E")
              end

            s
              .linearGradient(
                id: "med-tracker-logo-blue",
                x1: "120",
                y1: "8",
                x2: "151",
                y2: "68",
                gradientUnits: "userSpaceOnUse"
              ) do |gradient|
                gradient.stop(offset: "0%", stop_color: "#3278D5")
                gradient.stop(offset: "100%", stop_color: "#205CA8")
              end
          end

          s.rect(
            x: "19",
            y: "35",
            width: "30",
            height: "49",
            rx: "15",
            fill: "url(#med-tracker-logo-teal)",
            data_login_logo_part: "bar"
          )
          s.rect(
            x: "65",
            y: "19",
            width: "30",
            height: "65",
            rx: "15",
            fill: "url(#med-tracker-logo-teal)",
            data_login_logo_part: "bar"
          )
          s.path(
            d: "M111 24C111 15.7 117.7 9 126 9C134.3 9 141 15.7 141 24V61H111V24Z",
            fill: "url(#med-tracker-logo-blue)",
            data_login_logo_part: "bar"
          )
          s.circle(
            cx: "132",
            cy: "68",
            r: "28",
            fill: "url(#med-tracker-logo-teal)",
            data_login_logo_part: "check-badge"
          )
          s.path(
            d: "M119 68.5L128.2 77.7L146 57.5",
            stroke: "white",
            stroke_width: "7",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            data_login_logo_part: "check-mark"
          )
        end
      end

      private

      attr_reader :label
    end
  end
end
