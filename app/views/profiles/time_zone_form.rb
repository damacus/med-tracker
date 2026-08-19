# frozen_string_literal: true

module Views
  module Profiles
    class TimeZoneForm < Views::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::Routes

      def initialize(account:)
        super()
        @account = account
      end

      def view_template
        form_with(url: profile_path, method: :patch, class: 'space-y-5') do
          input(type: 'hidden', name: 'section', value: 'profile')
          input(type: 'hidden', name: 'profile_setting', value: 'time_zone')
          label(class: 'block text-sm font-bold text-foreground', for: 'account_time_zone') do
            t('profiles.time_zone.title')
          end
          render_time_zone_select
          render_actions
        end
      end

      private

      def render_time_zone_select
        select(id: 'account_time_zone', name: 'account[time_zone]', class: select_classes) do
          time_zone_options.each do |zone_name|
            option(value: zone_name, selected: zone_name == @account.preferred_time_zone) { zone_name }
          end
        end
      end

      def render_actions
        render SheetFooter.new(class: 'mt-6 flex flex-row justify-end gap-3') do
          m3_button(type: 'button', variant: :outlined, data: { action: 'click->ruby-ui--sheet-content#close' }) do
            t('ruby_ui.common.close')
          end
          m3_button(type: :submit, variant: :filled) { t('profiles.time_zone.save') }
        end
      end

      def time_zone_options
        ([@account.preferred_time_zone] + Account::TIME_ZONE_NAMES).select do |time_zone|
          Account.valid_time_zone?(time_zone)
        end.uniq
      end

      def select_classes
        'block min-h-11 w-full rounded-shape-sm border border-border bg-card px-3 py-2 text-sm text-foreground ' \
          'focus:border-primary focus:outline-none focus:ring-4 focus:ring-primary/5'
      end
    end
  end
end
