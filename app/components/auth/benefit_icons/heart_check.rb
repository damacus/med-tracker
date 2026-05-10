# frozen_string_literal: true

module Components
  module Auth
    module BenefitIcons
      class HeartCheck < Components::Base
        HEART_PATH = "M24 35C23.4 35 13 28.7 13 20.2C13 15.8 16 13 19.7 13C22 13 23.5 14.2 24 15.1C24.5 " \
          "14.2 26 13 28.3 13C32 13 35 15.8 35 20.2C35 28.7 24.6 35 24 35Z"

        def view_template
          svg(
            width: "48",
            height: "48",
            viewbox: "0 0 48 48",
            fill: "none",
            xmlns: "http://www.w3.org/2000/svg",
            data_login_benefit_icon: "stay-on-track",
            aria_hidden: "true"
          ) do |s|
            s.rect(width: "48", height: "48", rx: "14", fill: "#E0F7F4")
            s.path(
              d: HEART_PATH,
              stroke: "#109E91",
              stroke_width: "3",
              stroke_linejoin: "round"
            )
            s.path(
              d: "M18.5 22.8L22.4 26.7L30 19.2",
              stroke: "#109E91",
              stroke_width: "3",
              stroke_linecap: "round",
              stroke_linejoin: "round"
            )
          end
        end
      end
    end
  end
end
