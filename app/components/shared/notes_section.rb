# frozen_string_literal: true

module Components
  module Shared
    class NotesSection < Components::Base
      attr_reader :notes

      def initialize(notes:)
        @notes = notes
        super()
      end

      def view_template
        return if notes.blank?

        div(class: 'p-3 bg-blue-50 border border-blue-200 rounded-md') do
          Text(size: '2', class: 'text-blue-800') do
            span(class: 'font-semibold') { '📝 Notes: ' }
            plain notes
          end
        end
      end
    end
  end
end
