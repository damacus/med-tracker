# frozen_string_literal: true

module Views
  module Profiles
    class PersonalInfoContent < Views::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::Routes

      attr_reader :person, :account

      def initialize(person:, account:)
        super()
        @person = person
        @account = account
      end

      def view_template
        render CardContent.new(class: 'space-y-4') do
          render_basic_info_rows
          render_age_row if person.age
          render_capacity_info_rows
          render_time_zone_form
        end
      end

      private

      def render_basic_info_rows
        render_info_row('Name', person.name)
        render_info_row('Email', account.email)
        render_info_row('Time Zone', account.preferred_time_zone)
        render_info_row('Date of Birth', formatted_date_of_birth)
      end

      def render_age_row
        render_info_row('Age', person.age.to_s)
      end

      def render_capacity_info_rows
        render_info_row('Person Type', person.person_type.humanize)
        render_info_row('Has Capacity', person.has_capacity? ? 'Yes' : 'No')
      end

      def render_info_row(label, value)
        div(class: 'flex items-center justify-between border-b border-border py-3 last:border-0') do
          dt(class: 'text-sm font-medium text-on-surface-variant') { label }
          dd(class: 'text-sm text-foreground') { value || 'Not set' }
        end
      end

      def formatted_date_of_birth
        return 'Not set' unless person.date_of_birth

        person.date_of_birth.strftime('%B %d, %Y')
      end

      def render_time_zone_form
        div(class: 'rounded-shape-xl border border-outline-variant/70 bg-surface-container p-4') do
          form_with(url: profile_path, method: :patch, class: 'flex flex-col gap-4 sm:flex-row sm:items-end') do
            div(class: 'flex-1 space-y-2') do
              label(class: 'block text-sm font-bold text-foreground', for: 'account_time_zone') { 'Time Zone' }
              select(
                id: 'account_time_zone',
                name: 'account[time_zone]',
                class: time_zone_select_classes
              ) do
                time_zone_options.each do |zone_name|
                  option(value: zone_name, selected: zone_name == account.preferred_time_zone) { zone_name }
                end
              end
            end
            m3_button(type: :submit, variant: :tonal, size: :sm) { 'Save time zone' }
          end
        end
      end

      def time_zone_options
        Account::TIME_ZONE_NAMES
      end

      def time_zone_select_classes
        'block w-full rounded-shape-sm border border-border bg-card px-3 py-2 text-sm text-foreground ' \
          'focus:border-primary focus:outline-none focus:ring-4 focus:ring-primary/5'
      end
    end
  end
end
