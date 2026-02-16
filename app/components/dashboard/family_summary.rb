# frozen_string_literal: true

module Components
  module Dashboard
    class FamilySummary < Components::Base
      attr_reader :doses

      def initialize(doses:)
        @doses = doses
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-8') do
          header(class: 'mb-8') do
            render RubyUI::Heading.new(level: 1) { t('dashboard.title') }
            render RubyUI::Text.new(weight: 'muted') do
              t('dashboard.subtitle')
            end
          end

          div(class: 'space-y-4') do
            if doses.any?
              doses.each do |dose|
                render Components::Dashboard::TimelineItem.new(dose: dose)
              end
            else
              render_empty_state
            end
          end
        end
      end

      private

      def render_empty_state
        render RubyUI::Card.new(class: 'p-8 text-center') do
          render RubyUI::Text.new(weight: 'muted') do
            t('dashboard.empty_state')
          end
        end
      end
    end
  end
end
