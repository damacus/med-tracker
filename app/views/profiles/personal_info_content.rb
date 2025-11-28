# frozen_string_literal: true

module Views
  module Profiles
    class PersonalInfoContent < Views::Base
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
        end
      end

      private

      def render_basic_info_rows
        render_info_row('Name', person.name)
        render_info_row('Email', account.email)
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
        div(class: 'flex justify-between items-center py-3 border-b border-slate-200 last:border-0') do
          dt(class: 'text-sm font-medium text-slate-600') { label }
          dd(class: 'text-sm text-slate-900') { value || 'Not set' }
        end
      end

      def formatted_date_of_birth
        return 'Not set' unless person.date_of_birth

        person.date_of_birth.strftime('%B %d, %Y')
      end
    end
  end
end
