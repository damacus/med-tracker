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
          class: 'fixed left-0 top-16 bottom-0 z-40 flex w-16 flex-col items-center border-r ' \
                 'border-outline-variant/50 bg-surface-container-low py-4 md:hidden',
          aria: { label: t('layouts.mobile_rail.primary_navigation') },
          data: { testid: 'mobile-rail' }
        ) do
          nav(class: 'flex w-full flex-1 flex-col items-center gap-2 px-2', aria: { label: t('layouts.mobile_rail.primary_navigation') }) do
            primary_navigation_items.each do |item|
              render_nav_item(item)
            end
          end

          div(class: 'mt-auto flex w-full items-center justify-center px-2 pb-[max(1rem,env(safe-area-inset-bottom))] pt-4') do
            render_nav_item(profile_navigation_item)
          end
        end
      end

      private

      def render_nav_item(item)
        is_active = active_navigation_path?(item[:path])

        link_to(
          item[:path],
          class: 'flex min-h-[44px] min-w-[44px] items-center justify-center rounded-2xl p-1 no-underline',
          aria: {
            label: item[:label],
            current: is_active ? 'page' : nil
          }
        ) do
          div(
            class: 'flex h-12 w-12 items-center justify-center rounded-2xl transition-all state-layer ' \
                   "#{if is_active
                       'bg-secondary-container text-on-secondary-container shadow-sm'
                     else
                       'text-on-surface-variant hover:bg-surface-container-high hover:text-on-surface'
                     end}"
          ) do
            render item[:icon].new(size: 24)
          end
        end
      end
    end
  end
end
