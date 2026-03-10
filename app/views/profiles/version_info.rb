# frozen_string_literal: true

module Views
  module Profiles
    class VersionInfo < Components::Base
      include Phlex::Rails::Helpers::LinkTo

      def view_template
        div(class: 'flex flex-col items-center gap-3 px-2 py-6') do
          render RubyUI::Badge.new(
            variant: :outline,
            size: :sm,
            class: 'opacity-40 hover:opacity-100 transition-opacity font-mono'
          ) do
            link_to "v#{MedTracker::VERSION}",
                    "https://github.com/damacus/med-tracker/releases/tag/v#{MedTracker::VERSION}",
                    class: 'no-underline text-inherit',
                    target: '_blank',
                    rel: 'noopener'
          end

          div(class: 'flex gap-4 text-[10px] font-medium uppercase tracking-widest text-muted-foreground/50') do
            link_to 'Docs',
                    'https://damacus.github.io/med-tracker',
                    class: 'hover:text-foreground transition-colors no-underline',
                    target: '_blank',
                    rel: 'noopener'
            span(class: 'opacity-20') { '•' }
            link_to 'Release Notes',
                    "https://github.com/damacus/med-tracker/releases/tag/v#{MedTracker::VERSION}",
                    class: 'hover:text-foreground transition-colors no-underline',
                    target: '_blank',
                    rel: 'noopener'
          end
        end
      end
    end
  end
end
