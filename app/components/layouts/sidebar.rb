# frozen_string_literal: true

module Components
  module Layouts
    class Sidebar < Components::Base
      include Components::Layouts::CurrentUserContext
      include Components::Layouts::NavigationItems
      include Phlex::Rails::Helpers::LinkTo
      include Phlex::Rails::Helpers::Routes

      def view_template
        return unless authenticated?

        aside(
          class: 'app-sidebar fixed left-0 top-0 z-50 h-full w-64 flex-col border-r border-outline-variant ' \
                 'bg-surface-container-low px-4 py-8 text-on-surface-variant transition-all duration-500',
          data: { responsive_shell_role: 'sidebar' }
        ) do
          render_brand
          render_search_trigger
          render_navigation
          render_user_profile
        end
      end

      private

      def render_brand
        div(class: 'mb-12 flex w-full px-4 justify-center md:justify-start') do
          link_to root_path, class: 'group flex items-center gap-3 no-underline outline-none' do
            div(
              class: 'w-10 h-10 rounded-shape-lg bg-primary flex items-center justify-center ' \
                     'text-on-primary font-bold ' \
                     'text-xl shadow-elevation-2 group-hover:scale-110 transition-transform'
            ) do
              t('app.name').first
            end
            span(
              class: 'hidden md:block font-black text-xl tracking-tight text-foreground ' \
                     'group-hover:translate-x-1 transition-transform'
            ) do
              t('app.name')
            end
          end
        end
      end

      def render_navigation
        nav(class: 'flex-1 space-y-1 w-full') do
          primary_navigation_items.each do |item|
            render_nav_link(item)
          end

          admin_navigation_items.each do |item|
            render_nav_link(item)
          end
        end
      end

      def render_search_trigger
        button(
          type: 'button',
          class: 'mb-6 flex w-full items-center justify-center gap-3 rounded-md border border-outline-variant ' \
                 'bg-surface-container px-3 py-3 text-on-surface-variant transition-colors ' \
                 'hover:border-primary hover:text-on-surface focus-visible:outline-none focus-visible:ring-2 ' \
                 'focus-visible:ring-primary md:justify-start',
          aria: { label: t('global_search.open'), expanded: 'false', controls: 'global_search_panel' },
          data: { action: 'global-search#open', global_search_target: 'trigger' }
        ) do
          render Icons::Search.new(size: 20)
          span(class: 'hidden flex-1 text-left text-sm font-bold md:block') { t('global_search.open_short') }
          kbd(class: 'hidden rounded border border-outline-variant px-1.5 py-0.5 text-[10px] font-bold md:block') do
            t('global_search.shortcut_hint')
          end
        end
      end

      def render_nav_link(item)
        is_active = active_navigation_path?(item[:path])

        link_to item[:path],
                class: 'flex items-center gap-4 px-4 py-3 rounded-full transition-all ' \
                       'group no-underline relative state-layer ' \
                       "#{if is_active
                            'bg-secondary-container text-on-secondary-container'
                          else
                            'text-on-surface-variant hover:text-on-surface'
                          end}" do
          div(
            class: 'flex items-center justify-center z-10 ' \
                   "#{is_active ? 'text-on-secondary-container' : 'group-hover:scale-110 transition-transform'}"
          ) do
            render item[:icon].new(size: 24)
          end
          m3_text(
            variant: :label_large,
            class: 'hidden md:block z-10 ' \
                   "#{is_active ? 'text-on-secondary-container font-bold' : 'font-semibold'}"
          ) do
            item[:label]
          end
        end
      end

      def render_user_profile
        div(class: 'mt-auto w-full space-y-2 px-2') do
          link_to profile_navigation_item[:path],
                  class: 'flex items-center gap-3 rounded-full p-2 ' \
                         'transition-all no-underline group relative state-layer ' \
                         'bg-surface-container-high text-on-surface' do
            render Components::Shared::PersonAvatar.new(person: current_user.person, size: :sm, class: 'z-10')
            div(class: 'hidden md:block overflow-hidden z-10') do
              p(class: 'text-xs font-bold truncate text-foreground') do
                current_user_name || t('layouts.sidebar.user_fallback')
              end
              p(class: 'text-[10px] font-medium text-on-surface-variant') do
                t("activerecord.attributes.user.roles.#{current_user.role}", default: current_user.role.to_s.humanize)
              end
            end
          end

          form(action: '/logout', method: 'post', class: 'w-full', data_turbo: 'false') do
            input(type: 'hidden', name: 'authenticity_token', value: view_context.form_authenticity_token)
            button(
              type: 'submit',
              class: 'w-full flex items-center justify-center md:justify-start gap-4 px-4 py-3 rounded-full ' \
                     'text-on-surface-variant hover:text-error transition-all group relative state-layer',
              aria_label: t('layouts.sidebar.sign_out')
            ) do
              div(class: 'group-hover:scale-110 transition-transform flex items-center justify-center z-10') do
                render Icons::LogOut.new(size: 24)
              end
              m3_text(variant: :label_large, class: 'hidden md:block font-bold z-10') { t('layouts.sidebar.sign_out') }
            end
          end
        end
      end
    end
  end
end
