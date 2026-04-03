# frozen_string_literal: true

module Components
  module Admin
    module Users
      # Renders the user search and filter form
      class SearchForm < Components::Base
        attr_reader :search_params

        def initialize(search_params: {})
          @search_params = search_params
          super()
        end

        def view_template
          Card do
            CardContent(class: 'pt-6') do
              render RubyUI::Form.new(
                action: '/admin/users',
                method: :get,
                class: 'flex gap-4 items-end',
                data: { controller: 'filter-form', turbo_frame: 'admin-users-frame' }
              ) do
                render_search_field
                render_role_filter
                render_status_filter
                render_actions
              end
            end
          end
        end

        private

        def render_search_field
          div(class: 'flex-1') do
            render RubyUI::FormField.new do
              render RubyUI::FormFieldLabel.new(for: 'search') { t('admin.users.search.search') }
              render RubyUI::Input.new(
                type: :text,
                name: 'search',
                id: 'search',
                value: search_params[:search],
                placeholder: t('admin.users.search.search_placeholder'),
                data: { action: 'input->filter-form#submit' }
              )
            end
          end
        end

        def render_role_filter
          div(class: 'w-48') do
            render RubyUI::FormField.new do
              render RubyUI::FormFieldLabel.new(for: 'role') { t('admin.users.search.role') }
              select(
                name: 'role',
                id: 'role',
                class: select_classes,
                data: { action: 'change->filter-form#submit' }
              ) do
                option(value: '', selected: search_params[:role].blank?) { t('admin.users.search.all_roles') }
                User.roles.each_key do |role|
                  option(value: role, selected: search_params[:role] == role) { role.titleize }
                end
              end
            end
          end
        end

        def render_status_filter
          div(class: 'w-36') do
            render RubyUI::FormField.new do
              render RubyUI::FormFieldLabel.new(for: 'status') { t('admin.users.search.status') }
              select(
                name: 'status',
                id: 'status',
                class: select_classes,
                data: { action: 'change->filter-form#submit' }
              ) do
                option(value: '', selected: search_params[:status].blank?) { t('admin.users.search.all') }
                option(value: 'active', selected: search_params[:status] == 'active') { t('admin.users.user_row.active') }
                option(value: 'inactive', selected: search_params[:status] == 'inactive') { t('admin.users.user_row.inactive') }
                option(value: 'soft_deleted', selected: search_params[:status] == 'soft_deleted') { t('admin.users.user_row.soft_deleted') }
              end
            end
          end
        end

        def render_actions
          div(class: 'flex gap-2') do
            Button(type: :submit, variant: :primary, class: 'hidden') { t('admin.users.search.search') }
            Link(
              href: '/admin/users',
              variant: :outline,
              data: { turbo_frame: 'admin-users-frame' }
            ) { t('admin.users.search.clear') } if active_filters?
          end
        end

        def active_filters?
          search_params[:search].present? || search_params[:role].present? || search_params[:status].present?
        end
      end
    end
  end
end
