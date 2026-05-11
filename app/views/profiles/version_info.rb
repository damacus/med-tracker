# frozen_string_literal: true

module Views
  module Profiles
    class VersionInfo < Components::Base
      include Phlex::Rails::Helpers::LinkTo

      def view_template
        m3_card(class: 'border-border/70 shadow-elevation-2') do
          render CardHeader.new do
            render(CardTitle.new { t('profiles.version_info.title') })
            render(CardDescription.new { t('profiles.version_info.description') })
          end

          render CardContent.new(class: 'space-y-0') do
            render_system_rows
            render_docs_row
          end
        end
      end

      private

      def render_system_rows
        if git_metadata_mode?
          render_worktree_row
          render_commit_row
        else
          render_version_row
          render_release_notes_row
        end
      end

      def render_worktree_row
        div(class: 'flex items-center justify-between border-b border-border/50 py-3') do
          dt(class: 'text-sm font-medium text-on-surface-variant') { t('profiles.version_info.worktree') }
          dd(class: 'break-all text-right text-sm font-mono text-foreground') { system_metadata.worktree }
        end
      end

      def render_commit_row
        div(class: 'flex items-center justify-between border-b border-border/50 py-3') do
          dt(class: 'text-sm font-medium text-on-surface-variant') { t('profiles.version_info.commit') }
          dd(class: 'text-sm font-mono text-foreground') { system_metadata.commit }
        end
      end

      def render_version_row
        div(class: 'flex items-center justify-between border-b border-border/50 py-3') do
          dt(class: 'text-sm font-medium text-on-surface-variant') { t('profiles.version_info.app_version') }
          dd(class: 'text-sm font-mono text-foreground') { "v#{MedTracker::VERSION}" }
        end
      end

      def render_release_notes_row
        div(class: 'flex items-center justify-between border-b border-border/50 py-3') do
          dt(class: 'text-sm font-medium text-on-surface-variant') { t('profiles.version_info.release_notes') }
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
          dt(class: 'text-sm font-medium text-on-surface-variant') { t('profiles.version_info.documentation') }
          dd do
            link_to t('profiles.version_info.view_docs'),
                    'https://damacus.github.io/med-tracker',
                    class: 'text-sm font-medium text-primary hover:underline',
                    target: '_blank',
                    rel: 'noopener'
          end
        end
      end

      def system_metadata
        @system_metadata ||= SystemMetadata.current
      end

      def git_metadata_mode?
        Rails.env.development?
      end
    end
  end
end
