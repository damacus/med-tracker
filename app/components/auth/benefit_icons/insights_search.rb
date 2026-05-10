# frozen_string_literal: true

module Components
  module Auth
    module BenefitIcons
      class InsightsSearch < Components::Base
        PATH = "M400-320q100 0 170-70t70-170q0-100-70-170t-170-70q-100 0-170 70t-70 170q0 100 70 170t170 " \
          "70Zm-40-120v-280h80v280h-80Zm-140 0v-200h80v200h-80Zm280 0v-160h80v160h-80ZM824-80 " \
          "597-307q-41 32-91 49.5T400-240q-134 0-227-93T80-560q0-134 93-227t227-93q134 0 227 " \
          "93t93 227q0 56-17.5 106T653-363l227 227-56 56Z"

        def view_template
          svg(
            width: "48",
            height: "48",
            viewbox: "0 0 48 48",
            fill: "none",
            xmlns: "http://www.w3.org/2000/svg",
            data_login_benefit_icon: "insights",
            aria_hidden: "true"
          ) do |s|
            s.rect(width: "48", height: "48", rx: "14", fill: "#F2E7FF")
            s
              .svg(
                x: "12",
                y: "12",
                width: "24",
                height: "24",
                viewbox: "0 -960 960 960",
                xmlns: "http://www.w3.org/2000/svg"
              ) do |icon|
                icon.path(d: PATH, fill: "#9A5CF7")
              end
          end
        end
      end
    end
  end
end
