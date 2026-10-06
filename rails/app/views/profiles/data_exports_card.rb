# frozen_string_literal: true

module Views
  module Profiles
    class DataExportsCard < Views::Base
      include Phlex::Rails::Helpers::LinkTo
      include Phlex::Rails::Helpers::Routes

      def view_template
        m3_card(class: 'border-border/70 shadow-elevation-2', data: { testid: 'profile-data-exports-card' }) do
          m3_card_header do
            m3_card_title { 'Data backup' }
            m3_card_description { 'Download a copy of your medication and health records.' }
          end
          m3_card_content(class: 'space-y-4') do
            p(class: 'rounded-shape-lg border border-warning/40 bg-warning-container px-3 py-2 text-sm text-on-warning-container') do
              plain 'Unencrypted ZIP exports are not password protected. Store them somewhere private.'
            end
            div(class: 'flex flex-wrap gap-2') do
              m3_link(href: profile_data_export_path('health_data_json'), variant: :button) { 'Health data JSON' }
              m3_link(href: profile_data_export_path('backup_zip'), variant: :button) { 'Unencrypted ZIP' }
            end
          end
        end
      end
    end
  end
end
