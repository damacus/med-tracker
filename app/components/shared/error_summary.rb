# frozen_string_literal: true

module Components
  module Shared
    class ErrorSummary < Components::Base
      attr_reader :model, :resource_name

      def initialize(model:, resource_name:)
        @model = model
        @resource_name = resource_name
        super()
      end

      def view_template
        return unless model.errors.any?

        render RubyUI::Alert.new(variant: :destructive, class: 'mb-6') do
          div do
            Heading(level: 2, size: '3', class: 'font-semibold mb-2') do
              "#{model.errors.count} error(s) prohibited this #{resource_name} from being saved:"
            end
            ul(class: 'my-2 ml-6 list-disc [&>li]:mt-1') do
              model.errors.full_messages.each do |message|
                li { message }
              end
            end
          end
        end
      end
    end
  end
end
