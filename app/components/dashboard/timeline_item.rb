# frozen_string_literal: true

module Components
  module Dashboard
    class TimelineItem < Components::Base
      attr_reader :dose

      def initialize(dose:)
        @dose = dose
        super()
      end

      def view_template
        render RubyUI::Card.new(class: 'mb-4') do
          div(class: 'flex items-center justify-between p-4') do
            div(class: 'flex items-center space-x-4') do
              status_icon

              div do
                render RubyUI::Heading.new(level: 4) { dose[:source].medicine.name }
                render RubyUI::Text.new(size: '2', weight: 'muted') do
                  "#{dose[:person].name} • #{dose[:scheduled_at].strftime('%H:%M')}"
                end
              end
            end

            status_badge
          end
        end
      end

      private

      def status_icon
        case dose[:status]
        when :taken
          render Icons::CheckCircle.new(class: 'text-green-500 h-6 w-6')
        when :upcoming
          render Icons::Pill.new(class: 'text-blue-500 h-6 w-6')
        when :cooldown
          render Icons::AlertCircle.new(class: 'text-amber-500 h-6 w-6')
        else
          render Icons::AlertCircle.new(class: 'text-red-500 h-6 w-6')
        end
      end

      def status_badge
        variant = case dose[:status]
                  when :taken then :success
                  when :upcoming then :default
                  when :cooldown then :warning
                  else :destructive
                  end

        render RubyUI::Badge.new(variant: variant) do
          t("dashboard.statuses.#{dose[:status]}")
        end
      end
    end
  end
end
