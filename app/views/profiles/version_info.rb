# frozen_string_literal: true

module Views
  module Profiles
    class VersionInfo < Components::Base
      include Phlex::Rails::Helpers::LinkTo

      def view_template
        render Card.new(class: 'rounded-[2rem] border border-border/70 bg-card/95 shadow-[0_18px_45px_-32px_rgba(15,23,42,0.45)]') do
          render CardHeader.new do
            render(CardTitle.new { 'System Information' })
            render(CardDescription.new { 'Application version and resources' })
          end

          render CardContent.new(class: 'space-y-4') do
            render_info_row('App Version', "v#{MedTracker::VERSION}") do
              link_to "v#{MedTracker::VERSION}",
                      "https://github.com/damacus/med-tracker/releases/tag/v#{MedTracker::VERSION}",
                      class: 'text-primary hover:underline font-mono text-xs',
                      target: '_blank',
                      rel: 'noopener'
            end

            render_info_row('Release Notes', 'View latest changes') do
              link_to 'Release Notes',
                      "https://github.com/damacus/med-tracker/releases/tag/v#{MedTracker::VERSION}",
                      class: 'text-primary hover:underline text-xs',
                      target: '_blank',
                      rel: 'noopener'
            end

            render_info_row('Documentation', 'Get help and guides') do
              link_to 'View Docs',
                      'https://damacus.github.io/med-tracker',
                      class: 'text-primary hover:underline text-xs',
                      target: '_blank',
                      rel: 'noopener'
            end
          end
        end
      end

      private

      def render_info_row(label, _value, &)
        div(class: 'flex items-center justify-between border-b border-border/50 py-3 last:border-0') do
          dt(class: 'text-sm font-medium text-muted-foreground') { label }
          dd(class: 'text-sm text-foreground', &)
        end
      end
    end
  end
end
