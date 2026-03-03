# frozen_string_literal: true

module Components
  module Admin
    module Users
      # Admin users index view - displays list of users with search, filtering, and pagination
      class IndexView < Components::Base
        attr_reader :users, :search_params, :current_user, :pagy_obj

        def initialize(users:, search_params: {}, current_user: nil, pagy: nil)
          @users = users
          @search_params = search_params
          @current_user = current_user
          @pagy_obj = pagy
          super()
        end

        def view_template
          div(data: { testid: 'admin-users' },
              class: 'container mx-auto px-4 py-8 pb-24 md:pb-8 max-w-6xl space-y-8') do
            render_header
            render Components::Admin::Users::SearchForm.new(search_params: search_params)

            div(class: 'rounded-xl border border-border bg-card shadow-sm overflow-hidden') do
              render Components::Admin::Users::UsersTable.new(
                users: users,
                search_params: search_params,
                current_user: current_user
              )
            end

            render Components::Admin::Users::Pagination.new(pagy: pagy_obj, search_params: search_params) if pagy_obj
          end
        end

        private

        def render_header
          div(class: 'flex flex-col md:flex-row md:items-end justify-between gap-6 mb-12') do
            div do
              Text(size: '2', weight: 'muted', class: 'uppercase tracking-widest mb-1 block font-bold') do
                Time.current.strftime('%A, %b %d')
              end
              Heading(level: 1, size: '8', class: 'font-extrabold tracking-tight') do
                'User Management'
              end
              Text(weight: 'muted', class: 'mt-2 block') do
                'Review roles and access levels for everyone using MedTracker.'
              end
            end
            render RubyUI::Link.new(href: '/admin/users/new', variant: :primary, size: :lg, class: 'rounded-2xl shadow-lg shadow-primary/20') { 'New User' }
          end
        end
      end
    end
  end
end
