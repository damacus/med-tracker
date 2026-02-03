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
          div(data: { testid: 'admin-users' }, class: 'space-y-8 px-4 sm:px-6 lg:px-8') do
            render_header
            render Components::Admin::Users::SearchForm.new(search_params: search_params)
            render Components::Admin::Users::UsersTable.new(
              users: users,
              search_params: search_params,
              current_user: current_user
            )
            render Components::Admin::Users::Pagination.new(pagy: pagy_obj, search_params: search_params) if pagy_obj
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
      end
    end
  end
end
