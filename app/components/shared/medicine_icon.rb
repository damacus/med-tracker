# frozen_string_literal: true

module Components
  module Shared
    class MedicineIcon < Components::Base
      def view_template
        div(class: 'w-10 h-10 rounded-xl flex items-center justify-center bg-violet-100 text-violet-700 mb-2') do
          render Icons::Pill.new(size: 20)
        end
      end
    end
  end
end
