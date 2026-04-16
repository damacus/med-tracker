# frozen_string_literal: true

module Components
  module Admin
    module Users
      class FormView < Components::Base
        include Phlex::Rails::Helpers::FormWith
        include Phlex::Rails::Helpers::Pluralize

        attr_reader :user

        def initialize(user:)
          @user = user
          super()
        end

        def view_template
          div(class: 'container mx-auto px-4 py-12 max-w-2xl') do
            render_header
            render_form
          end
        end

        private

        def render_header
          div(class: 'text-center mb-10 space-y-2') do
            m3_heading(variant: :display_small, level: 1, class: 'font-black tracking-tight text-foreground') { form_title }
            m3_text(variant: :body_large, class: 'text-on-surface-variant font-medium') { t('admin.users.form.subtitle') }
          end
        end

        def form_title
          user.new_record? ? t('admin.users.form.create_title') : t('admin.users.form.edit_title')
        end

        def render_form
          form_with(
            model: [:admin, user],
            class: 'space-y-8',
            data: { testid: 'user-form' }
          ) do |form|
            render_errors if user.errors.any?
            render_form_fields(form)
            render_form_actions
          end
        end

        def render_errors
          render RubyUI::Alert.new(variant: :destructive, class: 'mb-8 rounded-shape-xl border-none shadow-elevation-1') do
            div(class: 'flex items-start gap-3') do
              render Icons::AlertCircle.new(size: 20)
              div do
                m3_heading(variant: :title_medium, level: 2, class: 'font-bold mb-1') do
                  plain "#{pluralize(user.errors.count, 'error')} prevented this user from being saved:"
                end
                ul(class: 'text-sm opacity-90 list-disc pl-4 space-y-1 font-medium') do
                  user.errors.full_messages.each do |message|
                    li { message }
                  end
                end
              end
            end
          end
        end

        def render_form_fields(form)
          m3_card(variant: :elevated, class: 'overflow-visible border-none shadow-elevation-3 rounded-[2.5rem]') do
            div(class: 'p-10 space-y-8') do
              render_person_fields(form)
              div(class: 'h-px bg-outline-variant w-full opacity-50')
              render_email_field(form)
              div(class: 'h-px bg-outline-variant w-full opacity-50')
              render_password_fields(form)
              div(class: 'h-px bg-outline-variant w-full opacity-50')
              render_role_field(form)
            end
          end
        end

        def render_person_fields(form)
          form.fields_for :person do |person_form|
            div(class: 'space-y-6') do
              render_name_field(person_form)
              render_date_of_birth_field(person_form)
              render_locations_field(person_form)
            end
          end
        end

        def render_name_field(_person_form)
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(for: 'user_person_attributes_name', class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1') do
              plain t('admin.users.form.name')
              span(class: 'text-error ml-0.5') { ' *' }
            end
            m3_input(
              type: :text,
              name: 'user[person_attributes][name]',
              id: 'user_person_attributes_name',
              value: user.person&.name,
              required: true,
              class: "rounded-md border-outline-variant bg-surface-container-lowest py-4 px-4 #{person_field_error_class(:name)}",
              **person_field_error_attributes(:name, input_id: 'user_person_attributes_name')
            )
            render_person_field_error(:name, input_id: 'user_person_attributes_name')
          end
        end

        def render_date_of_birth_field(_person_form)
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(for: 'user_person_attributes_date_of_birth', class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1') do
              plain t('admin.users.form.date_of_birth')
              span(class: 'text-error ml-0.5') { ' *' }
            end
            m3_input(
              type: :string,
              name: 'user[person_attributes][date_of_birth]',
              id: 'user_person_attributes_date_of_birth',
              value: user.person&.date_of_birth&.to_fs(:db),
              required: true,
              placeholder: 'YYYY-MM-DD',
              class: 'rounded-md border-outline-variant bg-surface-container-lowest py-4 px-4 transition-all',
              data: {
                controller: 'ruby-ui--calendar-input'
              },
              **person_field_error_attributes(:date_of_birth, input_id: 'user_person_attributes_date_of_birth')
            )
            render RubyUI::Calendar.new(
              input_id: '#user_person_attributes_date_of_birth',
              date_format: 'yyyy-MM-dd',
              class: 'rounded-md border shadow-elevation-2 bg-surface-container-high'
            )
            render_person_field_error(:date_of_birth, input_id: 'user_person_attributes_date_of_birth')
          end
        end

        def render_locations_field(_person_form)
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(for: 'user_person_attributes_location_ids', class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1') { t('admin.users.form.locations') }
            div(class: 'grid grid-cols-1 sm:grid-cols-2 gap-3 mt-2') do
              Location.find_each do |location|
                label(
                  class: 'flex items-center gap-3 p-4 rounded-xl border border-outline-variant bg-surface-container-low ' \
                         'hover:bg-surface-container-high cursor-pointer transition-all state-layer relative'
                ) do
                  input(
                    type: 'checkbox',
                    name: 'user[person_attributes][location_ids][]',
                    value: location.id,
                    id: "location_#{location.id}",
                    checked: user.person&.location_ids&.include?(location.id),
                    class: "z-10 #{checkbox_classes}"
                  )
                  span(class: 'z-10 font-bold text-foreground') { location.name }
                end
              end
            end
            # Hidden field to ensure location_ids is sent even if none selected (though validation will catch it)
            input(type: 'hidden', name: 'user[person_attributes][location_ids][]', value: '')
          end
        end

        def render_email_field(_form)
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(for: 'user_email_address', class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1') do
              plain t('admin.users.form.email_address')
              span(class: 'text-error ml-0.5') { ' *' }
            end
            m3_input(
              type: :email,
              name: 'user[email_address]',
              id: 'user_email_address',
              value: user.email_address,
              required: true,
              placeholder: 'email@example.com',
              class: "rounded-md border-outline-variant bg-surface-container-lowest py-4 px-4 #{field_error_class(user, :email_address)}",
              **field_error_attributes(user, :email_address, input_id: 'user_email_address')
            )
            render_field_error(user, :email_address, input_id: 'user_email_address')
          end
        end

        def render_password_fields(_form)
          div(class: 'grid grid-cols-1 sm:grid-cols-2 gap-6') do
            render_password_field
            render_password_confirmation_field
          end
        end

        def render_password_field
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(for: 'user_password', class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1') do
              plain t('admin.users.form.password')
              span(class: 'text-error ml-0.5') { ' *' } if user.new_record?
            end
            m3_input(
              type: :password,
              name: 'user[password]',
              id: 'user_password',
              required: user.new_record?,
              placeholder: '••••••••',
              class: "rounded-md border-outline-variant bg-surface-container-lowest py-4 px-4 #{field_error_class(user, :password)}",
              **field_error_attributes(user, :password, input_id: 'user_password')
            )
            render_field_error(user, :password, input_id: 'user_password')
          end
        end

        def render_password_confirmation_field
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(for: 'user_password_confirmation', class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1') do
              plain t('admin.users.form.password_confirmation')
              span(class: 'text-error ml-0.5') { ' *' } if user.new_record?
            end
            m3_input(
              type: :password,
              name: 'user[password_confirmation]',
              id: 'user_password_confirmation',
              placeholder: '••••••••',
              class: 'rounded-md border-outline-variant bg-surface-container-lowest py-4 px-4 transition-all',
              required: user.new_record?
            )
          end
        end

        def render_role_field(_form)
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(for: 'user_role_trigger', class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1') do
              plain t('admin.users.form.role')
              span(class: 'text-error ml-0.5') { ' *' }
            end
            render RubyUI::Combobox.new(class: 'w-full') do
              render RubyUI::ComboboxTrigger.new(
                id: 'role_trigger',
                placeholder: user.role.presence&.titleize || t('admin.users.form.select_role'),
                class: 'rounded-md border-outline-variant bg-surface-container-lowest py-4 px-4 transition-all'
              )

              render RubyUI::ComboboxPopover.new do
                render RubyUI::ComboboxSearchInput.new(
                  placeholder: t('admin.users.form.select_role')
                )

                render RubyUI::ComboboxList.new do
                  render(RubyUI::ComboboxEmptyState.new { t('admin.users.form.select_role') })

                  User.roles.each_key do |role|
                    render RubyUI::ComboboxItem.new do
                      render RubyUI::ComboboxRadio.new(
                        name: 'user[role]',
                        id: "user_role_#{role}",
                        value: role,
                        checked: user.role == role,
                        required: true
                      )
                      span { role.titleize }
                    end
                  end
                end
              end
            end
          end
        end

        def render_form_actions
          div(
            class: 'px-10 py-6 bg-surface-container-low border-t border-outline-variant/30 ' \
                   'flex items-center justify-between gap-4 rounded-b-[2.5rem]'
          ) do
            m3_link(href: admin_users_path, variant: :text, size: :lg,
                 class: 'font-bold text-on-surface-variant hover:text-foreground transition-all') do
              t('admin.users.form.cancel')
            end
            m3_button(type: :submit, variant: :filled, size: :lg,
                   class: 'px-8 rounded-shape-xl shadow-lg shadow-primary/20 transition-all') do
              user.new_record? ? t('admin.users.form.create_submit') : t('admin.users.form.update_submit')
            end
          end
        end

        def person_field_error_class(field)
          person = user.person
          return '' unless person

          field_error_class(person, field)
        end

        def person_field_error_attributes(field, input_id:)
          person = user.person
          return {} unless person

          field_error_attributes(person, field, input_id: input_id)
        end

        def render_person_field_error(field, input_id:)
          person = user.person
          return unless person

          render_field_error(person, field, input_id: input_id)
        end
      end
    end
  end
end