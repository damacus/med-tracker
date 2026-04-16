# frozen_string_literal: true

module Components
  module Shared
    class EmptyState < Components::Base
      attr_reader :title, :description, :icon

      def initialize(title:, description:, icon: :pill)
        @title = title
        @description = description
        @icon = icon
        super()
      end

      def view_template
        m3_card(
          class: 'p-10 text-center rounded-[2.5rem] border border-dashed border-border ' \
                 'bg-card'
        ) do
          div(
            class: 'w-16 h-16 rounded-full bg-secondary-container flex items-center justify-center ' \
                   'text-on-surface-variant mx-auto mb-5'
          ) do
            render_icon
          end
          m3_heading(level: 2, size: '5', class: 'font-bold tracking-tight mb-2') { title }
          m3_text(size: '3', class: 'text-on-surface-variant max-w-md mx-auto') { description }
        end
      end

      private

      def render_icon
        case icon
        when :search
          render Icons::Search.new(size: 28)
        else
          render Icons::Pill.new(size: 28)
        end
      end
    end
  end
end