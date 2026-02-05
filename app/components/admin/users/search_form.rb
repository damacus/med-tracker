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
              render RubyUI::Form.new(action: '/admin/users', method: :get, class: 'flex gap-4 items-end') do
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
        end

        def render_role_filter
          div(class: 'w-48') do
            render RubyUI::FormField.new do
              render RubyUI::FormFieldLabel.new(for: 'role') { 'Role' }
              Select do
                SelectInput(
                  name: 'role',
                  id: 'role',
                  value: search_params[:role]
                )
                SelectTrigger do
                  SelectValue(placeholder: 'All Roles') do
                    search_params[:role]&.titleize || 'All Roles'
                  end
                end
                SelectContent do
                  SelectItem(value: '') { 'All Roles' }
                  User.roles.each_key do |role|
                    SelectItem(value: role) { role.titleize }
                  end
                end
              end
            end
          end
        end

        def render_status_filter
          div(class: 'w-36') do
            render RubyUI::FormField.new do
              render RubyUI::FormFieldLabel.new(for: 'status') { 'Status' }
              Select do
                SelectInput(
                  name: 'status',
                  id: 'status',
                  value: search_params[:status]
                )
                SelectTrigger do
                  SelectValue(placeholder: 'All') do
                    search_params[:status]&.titleize || 'All'
                  end
                end
                SelectContent do
                  SelectItem(value: '') { 'All' }
                  SelectItem(value: 'active') { 'Active' }
                  SelectItem(value: 'inactive') { 'Inactive' }
                end
              end
            end
          end
        end

        def render_actions
          div(class: 'flex gap-2') do
            Button(type: :submit, variant: :primary) { 'Search' }
            Link(href: '/admin/users', variant: :outline) { 'Clear' } if active_filters?
          end
        end

        def active_filters?
          search_params[:search].present? || search_params[:role].present? || search_params[:status].present?
        end
      end
    end
  end
end
