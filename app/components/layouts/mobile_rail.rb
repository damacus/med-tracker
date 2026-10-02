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
            class: 'flex h-full w-full items-center justify-between gap-1',
            aria: { label: t('layouts.mobile_rail.primary_navigation') }
          ) do
            quick_navigation_items.each do |item|
              render_nav_item(item)
            end
          end
        end
      end

      private

      def quick_navigation_items
        available = mobile_shortcut_items.index_by { |item| item[:key] }
        shortcuts = Current.account&.preferred_mobile_shortcuts || Account::DEFAULT_MOBILE_SHORTCUTS
        shortcuts.filter_map { |key| available[key] }
      end

      def render_nav_item(item)
        is_active = active_navigation_path?(item[:path])

        link_to(
          item[:path],
          class: 'flex h-full min-h-11 min-w-0 flex-1 flex-col items-center justify-center gap-1 ' \
                 'rounded-2xl px-1 text-center no-underline focus-visible:outline-none ' \
                 'focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-inset',
          aria: {
            label: item[:label],
            current: is_active ? 'page' : nil
          }
        ) do
          div(
            class: 'flex h-[28px] w-16 max-w-full shrink-0 items-center justify-center rounded-2xl ' \
                   'transition-all state-layer ' \
                   "#{if is_active
                        'bg-secondary-container text-on-secondary-container shadow-sm'
                      else
                        'text-on-surface-variant hover:bg-surface-container-high hover:text-on-surface'
                      end}"
          ) do
            render item[:icon].new(size: 24, aria_hidden: 'true')
          end
          span(class: 'block h-auto min-h-[2.5em] w-full min-w-0 whitespace-normal break-words text-xs font-semibold ' \
                      'leading-tight text-on-surface') { item[:label] }
        end
      end
    end
  end
end
