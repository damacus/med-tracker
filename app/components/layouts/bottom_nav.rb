# frozen_string_literal: true

module Components
  module Layouts
    # Bottom navigation bar for mobile
    class BottomNav < Components::Base
      include Phlex::Rails::Helpers::LinkTo

      def view_template
        nav(class: 'mobile-nav fixed bottom-0 left-0 z-50 w-full h-16 bg-background border-t md:hidden ' \
                   'flex items-center justify-around px-4 pb-safe') do
          render_nav_item(root_path, Icons::Home, 'Home')
          render_nav_item(medicines_path, Icons::Pill, 'Medicines')
          render_nav_item(people_path, Icons::Users, 'People')
          render_nav_item(profile_path, Icons::User, 'Profile')
        end
      end

      private

      def render_nav_item(path, icon_class, label)
        link_to(path, class: 'flex flex-col items-center gap-1 text-muted-foreground hover:text-primary') do
          render icon_class.new(class: 'h-5 w-5')
          span(class: 'text-[10px] font-medium') { label }
        end
      end
    end
  end
end
