# frozen_string_literal: true

module Components
  module Icons
    class ActiveSchedules < Base
      def view_template
        svg(**attrs) do |s|
          s.path(
            d: "M200-640h560v-80H200v80Zm0 0v-80 80Zm0 560q-33 0-56.5-23.5T120-160v-560q0-33 23.5-56.5T200-800h40v-80h80v80h320v-80h80v80h40q33 0 56.5 23.5T840-720v227q-19-9-39-15t-41-9v-43H200v400h252q7 22 16.5 42T491-80H200Zm378.5-18.5Q520-157 520-240t58.5-141.5Q637-440 720-440t141.5 58.5Q920-323 920-240T861.5-98.5Q803-40 720-40T578.5-98.5ZM787-145l28-28-75-75v-112h-40v128l87 87Z"
          )
        end
      end

      private

      def default_attrs
        {
          xmlns: "http://www.w3.org/2000/svg",
          width: size.to_s,
          height: size.to_s,
          viewBox: "0 -960 960 960",
          fill: "currentColor",
          stroke: "none"
        }
      end
    end
  end
end
