# frozen_string_literal: true

module Components
  module Auth
    module BenefitIcons
      class ProgressPathPin < Components::Base
        def view_template
          svg(
            width: "48",
            height: "48",
            viewbox: "0 0 48 48",
            fill: "none",
            xmlns: "http://www.w3.org/2000/svg",
            data_login_benefit_icon: "progress",
            aria_hidden: "true"
          ) do |s|
            s.rect(width: "48", height: "48", rx: "14", fill: "#DFF7F1")
            s.path(
              d: "M21 14C16.6 14 13 17.4 13 21.8C13 27.8 21 35 21 35C21 35 29 27.8 29 21.8C29 17.4 25.4 14 21 14Z",
              fill: "#109E91"
            )
            s.path(d: "M21 18.5V25", stroke: "white", stroke_width: "3", stroke_linecap: "round")
            s.path(d: "M17.7 21.8H24.3", stroke: "white", stroke_width: "3", stroke_linecap: "round")
            s.path(
              d: "M28 34C31 35.5 34 35.2 36 33.5C38.1 31.7 38.8 29.3 39 27",
              stroke: "#109E91",
              stroke_width: "3",
              stroke_linecap: "round",
              stroke_dasharray: "1 6"
            )
            s.circle(cx: "39", cy: "25", r: "3", fill: "#109E91")
          end
        end
      end
    end
  end
end
