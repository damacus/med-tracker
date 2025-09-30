# frozen_string_literal: true

module Components
  module Admin
    module Users
      class IndexView < Components::Base
        attr_reader :users

        def initialize(users:)
          @users = users
          super()
        end

        def view_template
          div(data: { testid: 'admin-users' }, class: 'space-y-8') do
            render_header
            render_users_table
          end
        end

        private

        def render_header
          header(class: 'space-y-2') do
            h1(class: 'text-3xl font-semibold text-slate-900') { 'User Management' }
            p(class: 'text-slate-600') { 'Review roles and access levels for everyone using MedTracker.' }
          end
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
          tr(class: 'hover:bg-slate-50') do
            td(class: 'px-6 py-4 text-sm font-medium text-slate-900') { user.name }
            td(class: 'px-6 py-4 text-sm text-slate-600') { user.email_address }
            td(class: 'px-6 py-4 text-sm capitalize text-slate-600') { user.role }
          end
        end
      end
    end
  end
end
