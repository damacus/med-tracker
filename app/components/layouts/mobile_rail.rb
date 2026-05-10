# frozen_string_literal: true

module Components
  module Layouts
    class MobileRail < Components::Base
      include Components::Layouts::CurrentUserContext
      include Components::Layouts::NavigationItems
      include Phlex::Rails::Helpers::LinkTo

      def view_template
        return unless authenticated?

        nav(
          class: 'mobile-bottom-nav fixed bottom-[calc(1.5rem+env(safe-area-inset-bottom))] left-0 right-0 z-40 ' \
                 'mx-auto w-[90%] rounded-shape-xl border border-outline-variant/60 ' \
                 'bg-surface-container-lowest/95 px-2 py-2 shadow-elevation-3 backdrop-blur md:hidden',
          aria: { label: t('layouts.mobile_rail.primary_navigation') },
          data: { testid: 'mobile-bottom-nav' }
        ) do
          div(class: 'flex justify-between gap-1') do
            bottom_navigation_items.each { |item| render_nav_item(item) }
          end
        end
      end

      private

      def bottom_navigation_items
        [
          primary_navigation_items[0],
          primary_navigation_items[1],
          primary_navigation_items[5],
          profile_navigation_item
        ]
      end

      def render_nav_item(item)
        is_active = active_navigation_path?(item[:path])

        link_to(
          item[:path],
          class: 'flex min-h-[56px] min-w-[80px] flex-col items-center justify-center gap-1 rounded-shape-lg ' \
                 'px-2 py-1 text-xs font-bold no-underline transition-all',
          aria: {
            label: item[:label],
            current: is_active ? 'page' : nil
          }
        ) do
          div(
            class: 'flex h-8 min-w-12 items-center justify-center rounded-shape-full px-4 transition-all state-layer ' \
                   "#{if is_active
                        'bg-primary-container text-primary shadow-sm'
                      else
                        'text-on-surface-variant hover:bg-surface-container-high hover:text-on-surface'
                      end}"
          ) do
            render item[:icon].new(size: 22)
          end
          span(class: is_active ? 'text-primary' : 'text-on-surface-variant') { item[:label] }
        end
      end
    end
  end
end
