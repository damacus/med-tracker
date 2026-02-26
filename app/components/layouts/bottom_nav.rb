# frozen_string_literal: true

module Components
  module Layouts
    # Bottom navigation bar for mobile
    class BottomNav < Components::Base
      include Phlex::Rails::Helpers::LinkTo

      def view_template
        nav(
          class: 'mobile-nav fixed bottom-0 left-0 z-50 w-full h-20 bg-white/80 backdrop-blur-xl ' \
                 'border-t border-slate-100 md:hidden flex items-center justify-around px-6 pb-safe ' \
                 'transition-all duration-500'
        ) do
          render_nav_item(root_path, Icons::Home, 'Home')
          render_nav_item(medications_path, Icons::Pill, 'Inventory')
          render_nav_item(reports_path, Icons::AlertCircle, 'Reports')
          render_nav_item(edit_notification_preference_path, Icons::Bell, 'Alerts')
          render_nav_item(profile_path, Icons::User, 'Profile')
        end
      end

      private

      def render_nav_item(path, icon_class, label)
        is_active = current_page?(path)
        link_to(
          path,
          class: 'flex flex-col items-center justify-center gap-1.5 min-h-[44px] min-w-[44px] ' \
                 "transition-all no-underline #{is_active ? 'text-primary' : 'text-slate-400'}"
        ) do
          div(class: "transition-transform #{'scale-110' if is_active}") do
            render icon_class.new(size: 24)
          end
          span(class: "text-[10px] font-black uppercase tracking-widest #{is_active ? 'opacity-100' : 'opacity-40'}") do
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
