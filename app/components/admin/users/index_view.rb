# frozen_string_literal: true

module Components
  module Admin
    module Users
      # Admin users index view - displays list of users with search, filtering, and pagination
      class IndexView < Components::Base
        include Phlex::Rails::Helpers::TurboFrameTag

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

            turbo_frame_tag 'admin-users-frame', class: 'block space-y-8' do
              render Components::Admin::Users::SearchForm.new(search_params: search_params)

              div(class: 'rounded-[2rem] border border-border bg-card shadow-sm overflow-x-auto p-4') do
                render Components::Admin::Users::UsersTable.new(
                  users: users,
                  search_params: search_params,
                  current_user: current_user
                )
              end

              render Components::Admin::Users::Pagination.new(pagy: pagy_obj, search_params: search_params) if pagy_obj
            end
          end
        end

        private

        def render_header
          div(class: 'flex flex-col md:flex-row md:items-end justify-between gap-6 mb-12') do
            div do
              m3_text(size: '2', weight: 'muted', class: 'uppercase tracking-widest mb-1 block font-bold') do
                Time.current.strftime('%A, %b %d')
              end
              m3_heading(level: 1, size: '8', class: 'font-extrabold tracking-tight') do
                'User Management'
              end
              m3_text(weight: 'muted', class: 'mt-2 block') do
                'Review roles and access levels for everyone using MedTracker.'
              end
            end
            render RubyUI::Link.new(
              href: '/admin/users/new',
              variant: :primary,
              size: :lg,
              class: 'rounded-2xl shadow-lg shadow-primary/20',
              data: { turbo_frame: '_top' }
            ) { 'New User' }
          end
        end
      end
    end
  end
end
