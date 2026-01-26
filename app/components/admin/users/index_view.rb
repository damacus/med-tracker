# frozen_string_literal: true

module Components
  module Admin
    module Users
      class IndexView < Components::Base
        include Phlex::Rails::Helpers::ButtonTo
        include Phlex::Rails::Helpers::FormWith

        attr_reader :users, :search_params, :current_user, :pagy_obj

        def initialize(users:, search_params: {}, current_user: nil, pagy: nil)
          @users = users
          @search_params = search_params
          @current_user = current_user
          @pagy_obj = pagy
          super()
        end

        def view_template
          div(data: { testid: 'admin-users' }, class: 'space-y-8 px-4 sm:px-6 lg:px-8') do
            render_header
            render_search_form
            render_users_table
            render_pagination if pagy_obj
          end
        end

        private

        def render_header
          header(class: 'flex items-center justify-between') do
            div(class: 'space-y-2') do
              Heading(level: 1) { 'User Management' }
              Text(weight: 'muted') { 'Review roles and access levels for everyone using MedTracker.' }
            end
            render RubyUI::Link.new(href: '/admin/users/new', variant: :primary) { 'New User' }
          end
        end

        def render_search_form
          Card do
            CardContent(class: 'pt-6') do
              render RubyUI::Form.new(action: '/admin/users', method: :get, class: 'flex gap-4 items-end') do
                div(class: 'flex-1') do
                  render RubyUI::FormField.new do
                    render RubyUI::FormFieldLabel.new(for: 'search') { 'Search' }
                    render RubyUI::Input.new(
                      type: :text,
                      name: 'search',
                      id: 'search',
                      value: search_params[:search],
                      placeholder: 'Search by name or email...'
                    )
                  end
                end

                div(class: 'w-48') do
                  render RubyUI::FormField.new do
                    render RubyUI::FormFieldLabel.new(for: 'role') { 'Role' }
                    select(name: 'role', id: 'role', class: select_classes) do
                      option(value: '', selected: search_params[:role].blank?) { 'All Roles' }
                      User.roles.each_key do |role|
                        option(value: role, selected: search_params[:role] == role) { role.titleize }
                      end
                    end
                  end
                end

                div(class: 'w-36') do
                  render RubyUI::FormField.new do
                    render RubyUI::FormFieldLabel.new(for: 'status') { 'Status' }
                    select(name: 'status', id: 'status', class: select_classes) do
                      option(value: '', selected: search_params[:status].blank?) { 'All' }
                      option(value: 'active', selected: search_params[:status] == 'active') { 'Active' }
                      option(value: 'inactive', selected: search_params[:status] == 'inactive') { 'Inactive' }
                    end
                  end
                end

                div(class: 'flex gap-2') do
                  Button(type: :submit, variant: :primary) { 'Search' }
                  Link(href: '/admin/users', variant: :outline) { 'Clear' } if active_filters?
                end
              end
            end
          end
        end

        def active_filters?
          search_params[:search].present? || search_params[:role].present? || search_params[:status].present?
        end

        def render_users_table
          div(class: 'rounded-xl border border-border bg-card shadow-sm') do
            render RubyUI::Table.new do
              render_table_header
              render_table_body
            end
          end
        end

        def render_table_header
          render RubyUI::TableHeader.new do
            render RubyUI::TableRow.new do
              render(RubyUI::TableHead.new { render_sortable_header('Name', 'name') })
              render(RubyUI::TableHead.new { render_sortable_header('Email', 'email') })
              render(RubyUI::TableHead.new { render_sortable_header('Role', 'role') })
              render(RubyUI::TableHead.new { 'Status' })
              render RubyUI::TableHead.new(class: 'text-right') { 'Actions' }
            end
          end
        end

        def render_sortable_header(label, column)
          current_sort = search_params[:sort]
          current_direction = search_params[:direction] || 'asc'
          is_active = current_sort == column

          new_direction = is_active && current_direction == 'asc' ? 'desc' : 'asc'
          sort_params = search_params.to_h.merge(sort: column, direction: new_direction)

          Link(
            href: "/admin/users?#{sort_params.to_query}",
            variant: :link,
            class: sortable_header_class(is_active)
          ) do
            span { label }
            render_sort_indicator(is_active, current_direction)
          end
        end

        def sortable_header_class(is_active)
          base = 'inline-flex items-center gap-1 hover:text-slate-900 cursor-pointer'
          is_active ? "#{base} text-slate-900 font-semibold" : "#{base} text-slate-600"
        end

        def render_sort_indicator(is_active, direction)
          return unless is_active

          span(class: 'text-xs') do
            direction == 'asc' ? '↑' : '↓'
          end
        end

        def render_table_body
          render RubyUI::TableBody.new do
            users.each do |user|
              render_user_row(user)
            end
          end
        end

        def render_user_row(user)
          row_class = user.active? ? '' : 'opacity-60'
          render RubyUI::TableRow.new(class: row_class, data: { user_id: user.id }) do
            render RubyUI::TableCell.new(class: 'font-medium') { user.name }
            render(RubyUI::TableCell.new { user.email_address })
            render RubyUI::TableCell.new(class: 'capitalize') { user.role }
            render(RubyUI::TableCell.new { render_status_badge(user) })
            render RubyUI::TableCell.new(class: 'text-right space-x-2') do
              render RubyUI::Button.new(href: "/admin/users/#{user.id}/edit", variant: :outline, size: :sm) { 'Edit' }
              render_activation_button(user) unless user == current_user
            end
          end
        end

        def render_status_badge(user)
          if user.active?
            render RubyUI::Badge.new(variant: :green) { 'Active' }
          else
            render RubyUI::Badge.new(variant: :red) { 'Inactive' }
          end
        end

        def render_activation_button(user)
          if user.active?
            render_deactivate_dialog(user)
          else
            form_with(
              url: "/admin/users/#{user.id}/activate",
              method: :post,
              class: 'inline-block'
            ) do
              Button(
                type: :submit,
                variant: :link,
                class: 'text-green-600 hover:text-green-500 font-medium'
              ) { 'Activate' }
            end
          end
        end

        def render_deactivate_dialog(user)
          render RubyUI::AlertDialog.new do
            render RubyUI::AlertDialogTrigger.new do
              Button(variant: :destructive, size: :sm) { 'Deactivate' }
            end
            render RubyUI::AlertDialogContent.new do
              render RubyUI::AlertDialogHeader.new do
                render(RubyUI::AlertDialogTitle.new { 'Deactivate User Account' })
                render RubyUI::AlertDialogDescription.new do
                  "Are you sure you want to deactivate #{user.name}'s account? They will no longer be able to sign in."
                end
              end
              render RubyUI::AlertDialogFooter.new do
                render(RubyUI::AlertDialogCancel.new { 'Cancel' })
                form_with(url: "/admin/users/#{user.id}", method: :delete, class: 'inline') do
                  Button(variant: :destructive, type: :submit) { 'Deactivate' }
                end
              end
            end
          end
        end

        def render_pagination
          div(class: 'flex items-center justify-between border-t border-slate-200 bg-white px-4 py-3 sm:px-6') do
            div(class: 'flex flex-1 justify-between sm:hidden') do
              render_mobile_pagination
            end
            div(class: 'hidden sm:flex sm:flex-1 sm:items-center sm:justify-between') do
              render_pagination_info
              render_pagination_nav
            end
          end
        end

        def render_mobile_pagination
          if pagy_obj.previous
            Link(href: page_url(pagy_obj.previous), variant: :link, class: mobile_nav_class) { 'Previous' }
          else
            span(class: "#{mobile_nav_class} opacity-50 cursor-not-allowed") { 'Previous' }
          end

          if pagy_obj.next
            Link(href: page_url(pagy_obj.next), variant: :link, class: mobile_nav_class) { 'Next' }
          else
            span(class: "#{mobile_nav_class} opacity-50 cursor-not-allowed") { 'Next' }
          end
        end

        def mobile_nav_class
          'relative inline-flex items-center rounded-md border border-slate-300 bg-white ' \
            'px-4 py-2 text-sm font-medium text-slate-700 hover:bg-slate-50'
        end

        def render_pagination_info
          div(data: { testid: 'pagination-info' }) do
            Text(size: '2', class: 'text-slate-700') do
              plain 'Showing '
              span(class: 'font-medium') { pagy_obj.from.to_s }
              plain ' to '
              span(class: 'font-medium') { pagy_obj.to.to_s }
              plain ' of '
              span(class: 'font-medium') { pagy_obj.count.to_s }
              plain ' results'
            end
          end
        end

        def render_pagination_nav
          return if pagy_obj.pages <= 1

          nav(class: 'isolate inline-flex -space-x-px rounded-md shadow-sm', aria: { label: 'Pagination' }) do
            render_prev_button
            render_page_numbers
            render_next_button
          end
        end

        def render_prev_button
          if pagy_obj.previous
            Link(href: page_url(pagy_obj.previous), variant: :link, class: nav_button_class('rounded-l-md')) do
              span(class: 'sr-only') { 'Previous' }
              plain '‹'
            end
          else
            span(class: "#{nav_button_class('rounded-l-md')} opacity-50 cursor-not-allowed") do
              span(class: 'sr-only') { 'Previous' }
              plain '‹'
            end
          end
        end

        def render_next_button
          if pagy_obj.next
            Link(href: page_url(pagy_obj.next), variant: :link, class: nav_button_class('rounded-r-md')) do
              span(class: 'sr-only') { 'Next' }
              plain '›'
            end
          else
            span(class: "#{nav_button_class('rounded-r-md')} opacity-50 cursor-not-allowed") do
              span(class: 'sr-only') { 'Next' }
              plain '›'
            end
          end
        end

        def render_page_numbers
          pagy_obj.series.each do |item|
            case item
            when Integer
              Link(href: page_url(item), variant: :link, class: page_number_class(false)) { item.to_s }
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
          'relative inline-flex items-center px-2 py-2 text-slate-400 ring-1 ring-inset ' \
            "ring-slate-300 hover:bg-slate-50 focus:z-20 focus:outline-offset-0 #{extra}"
        end

        def page_number_class(current)
          base = 'relative inline-flex items-center px-4 py-2 text-sm font-semibold ' \
                 'ring-1 ring-inset ring-slate-300 focus:z-20 focus:outline-offset-0'
          current ? "#{base} z-10 bg-primary text-white" : "#{base} text-slate-900 hover:bg-slate-50"
        end

        def gap_class
          'relative inline-flex items-center px-4 py-2 text-sm font-semibold ' \
            'text-slate-700 ring-1 ring-inset ring-slate-300'
        end
      end
    end
  end
end
