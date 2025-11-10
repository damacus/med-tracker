# frozen_string_literal: true

module Components
  module Admin
    module Users
      class IndexView < Components::Base
        include Phlex::Rails::Helpers::FormWith

        attr_reader :users, :search_params

        def initialize(users:, search_params: {})
          @users = users
          @search_params = search_params
          super()
        end

        def view_template
          div(data: { testid: 'admin-users' }, class: 'space-y-8') do
            render_header
            render_search_form
            render_users_table
          end
        end

        private

        def render_header
          header(class: 'flex items-center justify-between') do
            div(class: 'space-y-2') do
              h1(class: 'text-3xl font-semibold text-slate-900') { 'User Management' }
              p(class: 'text-slate-600') { 'Review roles and access levels for everyone using MedTracker.' }
            end
            a(
              href: '/admin/users/new',
              class: 'inline-flex items-center justify-center rounded-md font-medium transition-colors ' \
                     'px-4 py-2 h-10 text-sm bg-primary text-primary-foreground hover:bg-primary/90'
            ) { 'New User' }
          end
        end

        def render_search_form
          Card do
            CardContent(class: 'pt-6') do
              form_with(url: '/admin/users', method: :get, class: 'flex gap-4 items-end') do
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
                    select(
                      name: 'role',
                      id: 'role',
                      class: input_classes
                    ) do
                      option(value: '', selected: search_params[:role].blank?) { 'All Roles' }
                      User.roles.each_key do |role|
                        option(value: role, selected: search_params[:role] == role) { role.titleize }
                      end
                    end
                  end
                end

                div(class: 'flex gap-2') do
                  render RubyUI::Button.new(type: :submit, variant: :primary) { 'Search' }
                  if search_params.present? && (search_params[:search].present? || search_params[:role].present?)
                    a(
                      href: '/admin/users',
                      class: 'inline-flex items-center justify-center rounded-md font-medium transition-colors ' \
                             'px-4 py-2 h-10 text-sm border border-input bg-background hover:bg-accent ' \
                             'hover:text-accent-foreground'
                    ) { 'Clear' }
                  end
                end
              end
            end
          end
        end

        def input_classes
          'w-full rounded-md border border-input bg-background px-3 py-2 text-sm ' \
            'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring'
        end

        def render_users_table
          div(class: 'overflow-hidden rounded-xl border border-slate-200 bg-white shadow-sm') do
            table(class: 'min-w-full divide-y divide-slate-100') do
              render_table_header
              render_table_body
            end
          end
        end

        def render_table_header
          thead(class: 'bg-slate-50') do
            tr do
              th(scope: 'col', class: 'px-6 py-3 text-left text-sm font-semibold text-slate-600') { 'Name' }
              th(scope: 'col', class: 'px-6 py-3 text-left text-sm font-semibold text-slate-600') { 'Email' }
              th(scope: 'col', class: 'px-6 py-3 text-left text-sm font-semibold text-slate-600') { 'Role' }
              th(scope: 'col', class: 'px-6 py-3 text-right text-sm font-semibold text-slate-600') { 'Actions' }
            end
          end
        end

        def render_table_body
          tbody(class: 'divide-y divide-slate-100') do
            users.each do |user|
              render_user_row(user)
            end
          end
        end

        def render_user_row(user)
          tr(class: 'hover:bg-slate-50', data: { user_id: user.id }) do
            td(class: 'px-6 py-4 text-sm font-medium text-slate-900') { user.name }
            td(class: 'px-6 py-4 text-sm text-slate-600') { user.email_address }
            td(class: 'px-6 py-4 text-sm capitalize text-slate-600') { user.role }
            td(class: 'px-6 py-4 text-sm text-right') do
              a(
                href: "/admin/users/#{user.id}/edit",
                class: 'text-primary hover:text-primary/80 font-medium'
              ) { 'Edit' }
            end
          end
        end
      end
    end
  end
end
