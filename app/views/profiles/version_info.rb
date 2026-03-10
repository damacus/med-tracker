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

          render CardContent.new(class: 'space-y-0') do
            render_version_row
            render_release_notes_row
            render_docs_row
          end
        end
      end

      private

      def render_version_row
        div(class: 'flex items-center justify-between border-b border-border/50 py-3') do
          dt(class: 'text-sm font-medium text-muted-foreground') { 'App Version' }
          dd(class: 'text-sm font-mono text-foreground') { "v#{MedTracker::VERSION}" }
        end
      end

      def render_release_notes_row
        div(class: 'flex items-center justify-between border-b border-border/50 py-3') do
          dt(class: 'text-sm font-medium text-muted-foreground') { 'Release Notes' }
          dd do
            link_to "v#{MedTracker::VERSION}",
                    "https://github.com/damacus/med-tracker/releases/tag/v#{MedTracker::VERSION}",
                    class: 'text-sm font-medium text-primary hover:underline',
                    target: '_blank',
                    rel: 'noopener'
          end
        end
      end

      def render_docs_row
        div(class: 'flex items-center justify-between py-3') do
          dt(class: 'text-sm font-medium text-muted-foreground') { 'Documentation' }
          dd do
            link_to 'View Docs',
                    'https://damacus.github.io/med-tracker',
                    class: 'text-sm font-medium text-primary hover:underline',
                    target: '_blank',
                    rel: 'noopener'
          end
        end
      end
    end
  end
end
