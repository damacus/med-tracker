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
          class: 'fixed left-0 top-0 h-full w-20 md:w-64 bg-white border-r border-slate-100 hidden ' \
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
        div(class: 'mb-12 px-2 w-full flex justify-center md:justify-start') do
          link_to root_path, class: 'group flex items-center gap-3 no-underline' do
            div(
              class: 'w-10 h-10 rounded-xl bg-primary flex items-center justify-center text-white font-bold ' \
                     'text-xl shadow-lg shadow-primary/20 group-hover:scale-110 transition-transform'
            ) do
              'M'
            end
            span(
              class: 'hidden md:block font-black text-xl tracking-tight text-foreground ' \
                     'group-hover:translate-x-1 transition-transform'
            ) do
              'MedTracker'
            end
          end
        end
      end

      def render_navigation
        nav(class: 'flex-1 space-y-2 w-full') do
          render_nav_link(root_path, Icons::Home, t('layouts.sidebar.dashboard'))
          render_nav_link(medicines_path, Icons::Pill, t('layouts.sidebar.inventory'))
          render_nav_link(locations_path, Icons::Home, t('layouts.sidebar.locations'))
          render_nav_link(medicine_finder_path, Icons::Search, t('layouts.sidebar.finder'))
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
                class: 'flex items-center gap-4 px-4 py-3 rounded-2xl transition-all group no-underline ' \
                       "#{if is_active
                            'bg-primary/5 text-primary'
                          else
                            'text-slate-400 hover:text-slate-600 hover:bg-slate-50'
                          end}" do
          div(
            class: 'flex items-center justify-center ' \
                   "#{is_active ? 'text-primary' : 'group-hover:scale-110 transition-transform'}"
          ) do
            render icon_class.new(size: 24)
          end
          span(class: "hidden md:block font-bold text-sm #{'text-primary' if is_active}") { label }
        end
      end

      def render_user_profile
        div(class: 'mt-auto w-full space-y-4 px-2') do
          link_to profile_path,
                  class: 'flex items-center gap-3 p-2 rounded-2xl border border-slate-100 bg-slate-50/50 ' \
                         'hover:bg-slate-100 transition-all no-underline group' do
            div(
              class: 'w-8 h-8 rounded-full bg-primary/10 flex items-center justify-center text-primary ' \
                     'font-bold text-xs overflow-hidden flex-shrink-0'
            ) do
              if current_user.person&.name.present?
                current_user.person.name.split.map(&:first).join.upcase
              else
                'U'
              end
            end
            div(class: 'hidden md:block overflow-hidden') do
              p(class: 'text-xs font-bold truncate text-foreground') { current_user.person&.name || 'User' }
              p(class: 'text-[10px] text-slate-400 font-medium') { current_user.role.to_s.humanize }
            end
          end

          form(action: '/logout', method: 'post', class: 'w-full') do
            input(type: 'hidden', name: 'authenticity_token', value: view_context.form_authenticity_token)
            button(
              type: 'submit',
              class: 'w-full flex items-center justify-center md:justify-start gap-4 px-4 py-3 rounded-2xl ' \
                     'text-slate-400 hover:text-destructive hover:bg-destructive/5 transition-all group'
            ) do
              div(class: 'group-hover:scale-110 transition-transform flex items-center justify-center') do
                render Icons::LogOut.new(size: 24)
              end
              span(class: 'hidden md:block font-bold text-sm') { t('layouts.sidebar.sign_out') }
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
