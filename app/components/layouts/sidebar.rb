# frozen_string_literal: true

module Components
  module Layouts
    class Sidebar < Components::Base
      include Phlex::Rails::Helpers::LinkTo
      include Phlex::Rails::Helpers::Routes

      attr_reader :current_user

      def initialize(current_user: nil)
        @current_user = current_user
        super()
      end

      def view_template
        return unless authenticated?

        aside(
          class: 'fixed left-0 top-0 hidden h-full w-20 border-r border-outline-variant bg-surface-container-low ' \
                 'text-on-surface-variant ' \
                 'md:w-64 ' \
                 'sm:flex flex-col items-center md:items-start py-8 px-4 z-50 transition-all duration-500'
        ) do
          render_brand
          render_navigation
          render_user_profile
        end
      end

      private

      def authenticated?
        @current_user.present?
      end

      def render_brand
        div(class: 'mb-12 px-4 w-full flex justify-center md:justify-start') do
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
          render_nav_link(root_path, Icons::Home, t('layouts.sidebar.dashboard'))
          render_nav_link(medications_path, Icons::Pill, t('layouts.sidebar.inventory'))
          render_nav_link(locations_path, Icons::Home, t('layouts.sidebar.locations'))
          render_nav_link(medication_finder_path, Icons::Search, t('layouts.sidebar.finder'))
          render_nav_link(people_path, Icons::Users, t('layouts.sidebar.people'))
          render_nav_link(reports_path, Icons::AlertCircle, t('layouts.sidebar.reports'))
          if current_user.administrator?
            render_nav_link(admin_root_path, Icons::Settings,
                            t('layouts.sidebar.administration'))
          end
        end
      end

      def render_nav_link(path, icon_class, label)
        is_active = current_page?(path)

        link_to path,
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
            render icon_class.new(size: 24)
          end
          m3_text(
            variant: :label_large,
            class: 'hidden md:block z-10 ' \
                   "#{is_active ? 'text-on-secondary-container font-bold' : 'font-semibold'}"
          ) do
            label
          end
        end
      end

      def render_user_profile
        div(class: 'mt-auto w-full space-y-2 px-2') do
          link_to profile_path,
                  class: 'flex items-center gap-3 rounded-full p-2 ' \
                         'transition-all no-underline group relative state-layer ' \
                         'bg-surface-container-high text-on-surface' do
            div(
              class: 'w-8 h-8 rounded-full bg-primary/10 flex items-center justify-center text-primary ' \
                     'font-bold text-xs overflow-hidden flex-shrink-0 z-10'
            ) do
              if current_user.person&.name.present?
                current_user.person.name.split.map(&:first).join.upcase
              else
                'U'
              end
            end
            div(class: 'hidden md:block overflow-hidden z-10') do
              p(class: 'text-xs font-bold truncate text-foreground') do
                current_user.person&.name || t('layouts.sidebar.user_fallback')
              end
              p(class: 'text-[10px] font-medium text-on-surface-variant') do
                t("activerecord.attributes.user.roles.#{current_user.role}", default: current_user.role.to_s.humanize)
              end
            end
          end

          form(action: '/logout', method: 'post', class: 'w-full') do
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

      def current_page?(path)
        return true if view_context.request.path == path
        return true if path != root_path && view_context.request.path.start_with?(path)

        false
      end
    end
  end
end
