# frozen_string_literal: true

module Components
  module Admin
    module Users
      # Renders pagination controls for the users list
      class Pagination < Components::Base
        attr_reader :pagy_obj, :search_params

        def initialize(pagy:, search_params: {})
          @pagy_obj = pagy
          @search_params = search_params
          super()
        end

        def view_template
          div(
            class: 'flex items-center justify-between border-t border-border ' \
                   'bg-card px-4 py-3 sm:px-6'
          ) do
            div(class: 'flex flex-1 justify-between sm:hidden') do
              render_mobile_pagination
            end
            div(class: 'hidden sm:flex sm:flex-1 sm:items-center sm:justify-between') do
              render_pagination_info
              render_pagination_nav
            end
          end
        end

        private

        def render_mobile_pagination
          if pagy_obj.previous
            Link(
              href: page_url(pagy_obj.previous),
              variant: :link,
              class: mobile_nav_class,
              data: { turbo_frame: 'admin-users-frame' }
            ) { t('admin.users.pagination.previous') }
          else
            span(class: "#{mobile_nav_class} opacity-50 cursor-not-allowed") { t('admin.users.pagination.previous') }
          end

          if pagy_obj.next
            Link(
              href: page_url(pagy_obj.next),
              variant: :link,
              class: mobile_nav_class,
              data: { turbo_frame: 'admin-users-frame' }
            ) { t('admin.users.pagination.next') }
          else
            span(class: "#{mobile_nav_class} opacity-50 cursor-not-allowed") { t('admin.users.pagination.next') }
          end
        end

        def mobile_nav_class
          'relative inline-flex items-center rounded-md border border-border bg-card ' \
            'px-4 py-2 text-sm font-medium text-foreground hover:bg-tertiary-container'
        end

        def render_pagination_info
          div(data: { testid: 'pagination-info' }) do
            m3_text(size: '2', class: 'text-foreground') do
              plain "#{t('admin.users.pagination.showing')} "
              span(class: 'font-medium') { pagy_obj.from.to_s }
              plain " #{t('admin.users.pagination.to')} "
              span(class: 'font-medium') { pagy_obj.to.to_s }
              plain " #{t('admin.users.pagination.of')} "
              span(class: 'font-medium') { pagy_obj.count.to_s }
              plain " #{t('admin.users.pagination.results')}"
            end
          end
        end

        def render_pagination_nav
          return if pagy_obj.pages <= 1

          nav(
            class: 'isolate inline-flex -space-x-px rounded-md shadow-sm',
            aria: { label: t('admin.users.pagination.label') }
          ) do
            render_prev_button
            render_page_numbers
            render_next_button
          end
        end

        def render_prev_button
          if pagy_obj.previous
            Link(
              href: page_url(pagy_obj.previous),
              variant: :link,
              class: nav_button_class('rounded-l-md'),
              data: { turbo_frame: 'admin-users-frame' }
            ) do
              span(class: 'sr-only') { t('admin.users.pagination.previous') }
              plain '‹'
            end
          else
            span(class: "#{nav_button_class('rounded-l-md')} opacity-50 cursor-not-allowed") do
              span(class: 'sr-only') { t('admin.users.pagination.previous') }
              plain '‹'
            end
          end
        end

        def render_next_button
          if pagy_obj.next
            Link(
              href: page_url(pagy_obj.next),
              variant: :link,
              class: nav_button_class('rounded-r-md'),
              data: { turbo_frame: 'admin-users-frame' }
            ) do
              span(class: 'sr-only') { t('admin.users.pagination.next') }
              plain '›'
            end
          else
            span(class: "#{nav_button_class('rounded-r-md')} opacity-50 cursor-not-allowed") do
              span(class: 'sr-only') { t('admin.users.pagination.next') }
              plain '›'
            end
          end
        end

        def render_page_numbers
          pagy_obj.series.each do |item|
            case item
            when Integer
              Link(
                href: page_url(item),
                variant: :link,
                class: page_number_class(false),
                data: { turbo_frame: 'admin-users-frame' }
              ) { item.to_s }
            when String
              span(class: page_number_class(true)) { item }
            when :gap
              span(class: gap_class) { '…' }
            end
          end
        end

        def page_url(page)
          params = search_params.to_h.merge(page: page)
          "/admin/users?#{params.to_query}"
        end

        def nav_button_class(extra = '')
          'relative inline-flex items-center px-2 py-2 text-on-surface-variant ring-1 ring-inset ' \
            "ring-border hover:bg-tertiary-container focus:z-20 focus:outline-offset-0 #{extra}"
        end

        def page_number_class(current)
          base = 'relative inline-flex items-center px-4 py-2 text-sm font-semibold ' \
                 'ring-1 ring-inset ring-border focus:z-20 focus:outline-offset-0'
          current ? "#{base} z-10 bg-primary text-white" : "#{base} text-foreground hover:bg-tertiary-container"
        end

        def gap_class
          'relative inline-flex items-center px-4 py-2 text-sm font-semibold ' \
            'text-foreground ring-1 ring-inset ring-border'
        end
      end
    end
  end
end