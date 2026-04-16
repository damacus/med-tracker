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
          m3_card(variant: :elevated, class: 'bg-surface-container-low border-none shadow-elevation-1') do
            m3_card_content(class: 'pt-8') do
              render RubyUI::Form.new(
                action: '/admin/users',
                method: :get,
                class: 'flex flex-col md:flex-row gap-6 items-end',
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
          div(class: 'flex-1 w-full') do
            div(class: 'space-y-2') do
              render RubyUI::FormFieldLabel.new(for: 'search', class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1') { t('admin.users.search.search') }
              m3_input(
                type: :text,
                name: 'search',
                id: 'search',
                value: search_params[:search],
                placeholder: t('admin.users.search.search_placeholder'),
                class: 'rounded-md border-outline-variant bg-surface-container-lowest py-4 px-4 transition-all',
                data: { action: 'input->filter-form#submit' }
              )
            end
          end
        end

        def render_role_filter
          div(class: 'w-full md:w-56') do
            div(class: 'space-y-2') do
              render RubyUI::FormFieldLabel.new(for: 'role_trigger', class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1') { t('admin.users.search.role') }
              render RubyUI::Combobox.new(class: 'w-full') do
                render RubyUI::ComboboxTrigger.new(
                  id: 'role_trigger',
                  placeholder: search_params[:role].presence&.titleize || t('admin.users.search.all_roles'),
                  class: 'rounded-md border-outline-variant bg-surface-container-lowest py-4 px-4 transition-all'
                )

                render RubyUI::ComboboxPopover.new do
                  render RubyUI::ComboboxSearchInput.new(
                    placeholder: t('admin.users.search.all_roles')
                  )

                  render RubyUI::ComboboxList.new do
                    render RubyUI::ComboboxItem.new do
                      render RubyUI::ComboboxRadio.new(
                        name: 'role',
                        id: 'role_all',
                        value: '',
                        checked: search_params[:role].blank?,
                        data: { action: 'change->filter-form#submit' }
                      )
                      span { t('admin.users.search.all_roles') }
                    end

                    User.roles.each_key do |role|
                      render RubyUI::ComboboxItem.new do
                        render RubyUI::ComboboxRadio.new(
                          name: 'role',
                          id: "role_#{role}",
                          value: role,
                          checked: search_params[:role] == role,
                          data: { action: 'change->filter-form#submit' }
                        )
                        span { role.titleize }
                      end
                    end
                  end
                end
              end
            end
          end
        end

        def render_status_filter
          div(class: 'w-full md:w-48') do
            div(class: 'space-y-2') do
              render RubyUI::FormFieldLabel.new(for: 'status_trigger', class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1') { t('admin.users.search.status') }
              render RubyUI::Combobox.new(class: 'w-full') do
                render RubyUI::ComboboxTrigger.new(
                  id: 'status_trigger',
                  placeholder: search_params[:status].presence&.humanize || t('admin.users.search.all'),
                  class: 'rounded-md border-outline-variant bg-surface-container-lowest py-4 px-4 transition-all'
                )

                render RubyUI::ComboboxPopover.new do
                  render RubyUI::ComboboxSearchInput.new(
                    placeholder: t('admin.users.search.status')
                  )

                  render RubyUI::ComboboxList.new do
                    render RubyUI::ComboboxItem.new do
                      render RubyUI::ComboboxRadio.new(
                        name: 'status',
                        id: 'status_all',
                        value: '',
                        checked: search_params[:status].blank?,
                        data: { action: 'change->filter-form#submit' }
                      )
                      span { t('admin.users.search.all') }
                    end

                    %w[active inactive soft_deleted].each do |status|
                      render RubyUI::ComboboxItem.new do
                        render RubyUI::ComboboxRadio.new(
                          name: 'status',
                          id: "status_#{status}",
                          value: status,
                          checked: search_params[:status] == status,
                          data: { action: 'change->filter-form#submit' }
                        )
                        span { t("admin.users.user_row.#{status}", default: status.humanize) }
                      end
                    end
                  end
                end
              end
            end
          end
        end

        def render_actions
          div(class: 'flex gap-3 pb-1') do
            m3_button(type: :submit, variant: :filled, class: 'hidden') { t('admin.users.search.search') }
            if active_filters?
              m3_link(
                href: '/admin/users',
                variant: :text,
                size: :sm,
                class: 'font-bold text-on-surface-variant hover:text-foreground transition-all',
                data: { turbo_frame: 'admin-users-frame' }
              ) { t('admin.users.search.clear') }
            end
          end
        end

        def active_filters?
          search_params[:search].present? || search_params[:role].present? || search_params[:status].present?
        end
      end
    end
  end
end
