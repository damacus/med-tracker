# frozen_string_literal: true

module Components
  module Layouts
    class FloatingActionMenu < Components::Base
      include Components::Layouts::CurrentUserContext

      def view_template
        return unless authenticated?
        return unless visible?

        div(
          class: 'group/floating-action-menu md:hidden',
          data: {
            controller: 'floating-action-menu',
            open: 'false',
            action: floating_action_menu_actions
          }
        ) do
          div(
            class: 'fixed inset-0 z-40 bg-foreground/5 transition-opacity duration-200 ' \
                   'group-data-[open=false]/floating-action-menu:pointer-events-none ' \
                   'group-data-[open=false]/floating-action-menu:opacity-0 ' \
                   'group-data-[open=true]/floating-action-menu:opacity-100',
            data: { testid: 'floating-action-backdrop', action: 'click->floating-action-menu#close' }
          )

          div(
            class: 'fixed bottom-[calc(1.5rem+env(safe-area-inset-bottom))] ' \
                   'right-[calc(1rem+env(safe-area-inset-right))] ' \
                   'z-50 flex flex-col items-end gap-3'
          ) do
            div(
              id: 'floating-action-menu-items',
              class: 'flex flex-col items-end gap-3',
              hidden: true,
              aria: { hidden: 'true' },
              data: { floating_action_menu_target: 'menu', testid: 'floating-action-menu-items' }
            ) do
              floating_action_items.each do |item|
                render_action_item(item)
              end
            end

            button(
              type: 'button',
              class: 'flex h-14 w-14 items-center justify-center rounded-full bg-primary text-on-primary ' \
                     'shadow-xl shadow-primary/30 transition-all hover:scale-105 focus-visible:outline-none ' \
                     'focus-visible:ring-2 focus-visible:ring-primary',
              aria: {
                label: t('layouts.floating_action_menu.open'),
                expanded: 'false',
                controls: 'floating-action-menu-items'
              },
              data: {
                action: 'floating-action-menu#toggle',
                floating_action_menu_close_label: t('layouts.floating_action_menu.close'),
                floating_action_menu_open_label: t('layouts.floating_action_menu.open'),
                floating_action_menu_target: 'toggle',
                testid: 'floating-action-menu-toggle'
              }
            ) do
              span(class: 'group-data-[open=true]/floating-action-menu:hidden') do
                render Icons::Plus.new(size: 28)
              end
              span(class: 'hidden group-data-[open=true]/floating-action-menu:block') do
                render Icons::X.new(size: 24)
              end
            end
          end
        end
      end

      private

      def floating_action_menu_actions
        'click@window->floating-action-menu#closeOnOutsideClick ' \
          'turbo:before-visit@window->floating-action-menu#close'
      end

      def visible?
        request_path = view_context.request.path

        [
          root_path,
          dashboard_path,
          people_path,
          medications_path,
          locations_path,
          schedules_path
        ].include?(request_path)
      end

      def floating_action_items
        items = [
          {
            label: t('dashboard.quick_actions.add_medication'),
            path: add_medication_path,
            icon: Icons::Pill,
            icon_classes: 'bg-primary text-on-primary',
            data: { turbo_frame: 'modal' }
          }
        ]

        if view_context.policy(Person.new).new?
          items << {
            label: t('dashboard.quick_actions.add_person'),
            path: new_person_path,
            icon: Icons::Users,
            icon_classes: 'bg-teal-600 text-white'
          }
        end

        if view_context.policy(Location.new).new?
          items << {
            label: t('locations.index.add_location'),
            path: new_location_path,
            icon: Icons::Home,
            icon_classes: 'bg-blue-600 text-white'
          }
        end

        items
      end

      def render_action_item(item)
        m3_link(
          href: item[:path],
          variant: :text,
          class: 'flex items-center gap-3 rounded-full p-0 text-on-surface no-underline',
          aria: { label: item[:label] },
          data: item.fetch(:data, {}).merge(
            action: 'click->floating-action-menu#closeAndNavigate',
            floating_action_menu_target: 'item'
          )
        ) do
          span(class: 'rounded-full bg-surface-container-high px-4 py-2 text-sm font-bold shadow-elevation-2') do
            item[:label]
          end
          span(
            class: 'flex h-12 w-12 items-center justify-center rounded-full shadow-elevation-2 ' \
                   "#{item[:icon_classes]}"
          ) do
            render item[:icon].new(size: 22)
          end
        end
      end
    end
  end
end
