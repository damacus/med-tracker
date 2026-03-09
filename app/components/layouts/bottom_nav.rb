# frozen_string_literal: true

module Components
  module Layouts
    # Bottom navigation bar for mobile
    class BottomNav < Components::Base
      include Phlex::Rails::Helpers::LinkTo

      def view_template
        nav(
          class: 'mobile-nav fixed bottom-0 left-0 z-50 flex h-20 w-full items-center justify-around ' \
                 'border-t border-border bg-background/80 px-6 pb-safe backdrop-blur-xl md:hidden ' \
                 'transition-all duration-500'
        ) do
          render_nav_item(root_path, Icons::Home, 'Home')
          render_nav_item(medications_path, Icons::Pill, 'Inventory')
          render_nav_item(reports_path, Icons::AlertCircle, 'Reports')
          render_nav_item(profile_path, Icons::User, 'Profile')
          render_version_badge
        end
      end

      private

      def render_nav_item(path, icon_class, label)
        is_active = current_page?(path)
        link_to(
          path,
          class: 'flex flex-col items-center justify-center gap-1.5 min-h-[44px] min-w-[44px] ' \
                 "transition-all no-underline #{is_active ? 'text-primary' : 'text-muted-foreground'}"
        ) do
          div(class: "transition-transform #{'scale-110' if is_active}") do
            render icon_class.new(size: 24)
          end
          span(class: 'text-[10px] font-black uppercase tracking-widest') do
            label
          end
        end
      end

      def current_page?(path)
        return true if view_context.request.path == path
        return true if path != root_path && view_context.request.path.start_with?(path)

        false
      end
    end
  end
end
