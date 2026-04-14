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
        Card(
          class: 'p-10 text-center rounded-[2.5rem] border border-dashed border-border ' \
                 'bg-card'
        ) do
          div(
            class: 'w-16 h-16 rounded-full bg-muted flex items-center justify-center ' \
                   'text-muted-foreground mx-auto mb-5'
          ) do
            render_icon
          end
          Heading(level: 2, size: '5', class: 'font-bold tracking-tight mb-2') { title }
          Text(size: '3', class: 'text-muted-foreground max-w-md mx-auto') { description }
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
