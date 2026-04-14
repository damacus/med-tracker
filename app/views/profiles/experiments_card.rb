# frozen_string_literal: true

module Views
  module Profiles
    class ExperimentsCard < Views::Base
      include Phlex::Rails::Helpers::FormWith

      WIZARD_OPTIONS = [
        {
          value: 'fullpage',
          label: 'Full page',
          description: 'Opens as a dedicated page. Best for focused data entry.'
        },
        {
          value: 'modal',
          label: 'Modal',
          description: 'Pops up over the current page. Quick without losing context.'
        },
        {
          value: 'slideover',
          label: 'Slide-over',
          description: 'Slides in from the right. See your list while editing.'
        }
      ].freeze

      attr_reader :user

      def initialize(user:)
        @user = user
        super()
      end

      def view_template
        render Card.new(
          id: 'experiments-card',
          class: 'overflow-hidden border-border/70 shadow-elevation-2'
        ) do
          render CardHeader.new(class: 'border-b border-border/60 pb-6') do
            div(class: 'flex items-center gap-3 mb-2') do
              render Components::Icons::Sparkles.new(size: 20, class: 'text-primary')
              render(CardTitle.new { 'Experiments' })
            end
            render(CardDescription.new { 'Try new UI features before they ship.' })
          end
          render CardContent.new(class: 'px-5 py-6 sm:px-6') do
            render_wizard_section
          end
        end
      end

      private

      def render_wizard_section
        div(class: 'space-y-4') do
          p(class: 'text-[0.7rem] font-semibold uppercase tracking-[0.28em] text-muted-foreground') do
            'Add Medication Wizard Style'
          end
          form_with(
            url: experiments_profile_path,
            method: :patch,
            data: { controller: 'auto-submit' }
          ) do
            div(class: 'grid gap-3') do
              WIZARD_OPTIONS.each do |option|
                render_option(option)
              end
            end
          end
        end
      end

      def render_option(option)
        selected = user.wizard_variant == option[:value]
        border_class = selected ? 'border-primary ring-2 ring-primary/20 shadow-elevation-2' : 'border-border/70'

        label(
          class: "flex items-start gap-4 rounded-shape-xl border #{border_class} bg-popover " \
                 'p-4 shadow-elevation-1 cursor-pointer transition-all hover:border-primary/50 hover:bg-accent/30'
        ) do
          input(
            type: 'radio',
            name: 'user[wizard_variant]',
            value: option[:value],
            checked: selected,
            class: 'sr-only',
            data: { action: 'change->auto-submit#submit' }
          )
          div(class: 'flex-1 min-w-0') do
            div(class: 'flex items-center justify-between') do
              span(class: 'text-sm font-semibold text-foreground') { option[:label] }
              render Components::Icons::Check.new(
                size: 16,
                class: selected ? 'text-primary' : 'invisible'
              )
            end
            p(class: 'text-xs text-muted-foreground mt-0.5') { option[:description] }
          end
        end
      end
    end
  end
end
