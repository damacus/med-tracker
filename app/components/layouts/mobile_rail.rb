# frozen_string_literal: true

module Components
  module Layouts
    class MobileRail < Components::Base
      include Components::Layouts::CurrentUserContext
      include Components::Layouts::NavigationItems
      include Phlex::Rails::Helpers::LinkTo

      def view_template
        return unless authenticated?

        aside(
          class: 'app-mobile-rail fixed inset-x-0 bottom-0 z-40 flex h-20 items-center border-t ' \
                 'border-outline-variant/50 bg-surface-container-low px-2 ' \
                 'pb-[max(0.5rem,env(safe-area-inset-bottom))] pt-2 md:hidden',
          aria: { label: t('layouts.mobile_rail.primary_navigation') },
          data: { testid: 'mobile-rail', responsive_shell_role: 'mobile-rail' }
        ) do
          nav(
            class: 'flex w-full items-center justify-between gap-0',
            aria: { label: t('layouts.mobile_rail.primary_navigation') }
          ) do
            primary_navigation_items.each do |item|
              render_nav_item(item)
            end
            render_nav_item(profile_navigation_item)
          end
        end
      end

      private

      def render_nav_item(item)
        is_active = active_navigation_path?(item[:path])

        link_to(
          item[:path],
          class: 'flex h-11 w-11 shrink-0 items-center justify-center rounded-2xl no-underline',
          aria: {
            label: item[:label],
            current: is_active ? 'page' : nil
          }
        ) do
          div(
            class: 'flex h-11 w-11 items-center justify-center rounded-2xl transition-all state-layer ' \
                   "#{if is_active
                        'bg-secondary-container text-on-secondary-container shadow-sm'
                      else
                        'text-on-surface-variant hover:bg-surface-container-high hover:text-on-surface'
                      end}"
          ) do
            render item[:icon].new(size: 24, aria_hidden: 'true')
          end
        end
      end
    end
  end
end
