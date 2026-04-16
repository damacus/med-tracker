# frozen_string_literal: true

module Components
  module Layouts
    # Bottom navigation bar for mobile
    class BottomNav < Components::Base
      include Phlex::Rails::Helpers::LinkTo

      def view_template
        nav(
          class: 'mobile-nav fixed bottom-0 left-0 z-50 flex h-20 w-full items-center justify-around ' \
                 'border-t border-outline-variant bg-surface-container px-6 pb-safe md:hidden ' \
                 'transition-all duration-500'
        ) do
          render_nav_item(root_path, Icons::Home, 'Home')
          render_nav_item(medications_path, Icons::Pill, 'Inventory')
          render_nav_item(reports_path, Icons::AlertCircle, 'Reports')
          render_nav_item(profile_path, Icons::User, 'Profile')
        end
      end

      private

      def render_nav_item(path, icon_class, label)
        is_active = current_page?(path)
        link_to(
          path,
          class: 'flex flex-col items-center justify-center gap-1.5 min-h-[44px] min-w-[44px] ' \
                 'transition-all no-underline relative state-layer rounded-xl ' \
                 "#{is_active ? 'text-primary' : 'text-on-surface-variant hover:text-on-surface'}"
        ) do
          div(class: "relative flex items-center justify-center transition-all z-10 #{'px-5 py-1 bg-secondary-container rounded-shape-full text-on-secondary-container' if is_active}") do
            render icon_class.new(size: 24)
          end
          span(class: "text-[11px] z-10 #{is_active ? 'font-black' : 'font-bold'}") do
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
