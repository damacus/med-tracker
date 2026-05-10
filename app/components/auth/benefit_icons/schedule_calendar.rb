# frozen_string_literal: true

module Components
  module Auth
    module BenefitIcons
      class ScheduleCalendar < Components::Base
        PATH = "M200-80q-33 0-56.5-23.5T120-160v-560q0-33 23.5-56.5T200-800h40v-80h80v80h320v-80h80v80h40q33 " \
          "0 56.5 23.5T840-720v255l-80 80v-175H200v400h248l80 80H200Zm0-560h560v-80H200v80Zm0 " \
          "0v-80 80Zm462 580L520-202l56-56 85 85 170-170 56 57L662-60Z"

        def view_template
          svg(
            width: "48",
            height: "48",
            viewbox: "0 0 48 48",
            fill: "none",
            xmlns: "http://www.w3.org/2000/svg",
            data_login_benefit_icon: "schedule",
            aria_hidden: "true"
          ) do |s|
            s.rect(width: "48", height: "48", rx: "14", fill: "#E7F0FF")
            s
              .svg(
                x: "12",
                y: "12",
                width: "24",
                height: "24",
                viewbox: "0 -960 960 960",
                xmlns: "http://www.w3.org/2000/svg"
              ) do |icon|
                icon.path(d: PATH, fill: "#2F7DE1")
              end
          end
        end
      end
    end
  end
end
