# frozen_string_literal: true

module Components
  module MedicationReviews
    class IndexView < Components::Base
      def initialize(prompts:, hidden_count:, show_hidden:)
        @prompts = prompts
        @hidden_count = hidden_count
        @show_hidden = show_hidden
        super()
      end

      def view_template
        div(class: 'container mx-auto max-w-6xl px-4 py-10 pb-24 md:py-12', data: { testid: 'medicine-reviews' }) do
          render_header
          render_boundary
          render_hidden_notice if hidden_count.positive?
          render_prompts
        end
      end

      private

      attr_reader :prompts, :hidden_count, :show_hidden

      def render_header
        header(class: 'flex flex-col gap-6 border-b border-border pb-8 md:flex-row md:items-end md:justify-between') do
          div(class: 'flex items-center gap-5') do
            div(class: 'flex h-16 w-16 shrink-0 items-center justify-center rounded-shape-lg ' \
                       'bg-secondary-container text-on-secondary-container') do
              render Icons::FileText.new(size: 30)
            end
            div do
              m3_text(variant: :label_medium,
                      class: 'block font-bold uppercase tracking-widest text-on-surface-variant') do
                t('medication_reviews.eyebrow')
              end
              m3_heading(level: 1, size: '8', class: 'font-black') { t('medication_reviews.title') }
              m3_text(variant: :body_large, class: 'mt-1 block max-w-2xl text-on-surface-variant') do
                t('medication_reviews.subtitle')
              end
            end
          end
          m3_link(href: medication_review_report_path, variant: :filled, class: 'gap-2 self-start md:self-auto') do
            render Icons::FileText.new(size: 18)
            plain t('medication_reviews.export_pdf')
          end
        end
      end

      def render_boundary
        aside(class: 'my-8 border-l-4 border-primary bg-surface-container-low px-5 py-4', role: 'note') do
          m3_text(variant: :body_medium, class: 'block font-medium leading-relaxed') do
            t('medication_reviews.boundary')
          end
        end
      end

      def render_hidden_notice
        div(class: 'mb-8 flex flex-wrap items-center justify-between gap-3 border-y border-border py-4',
            data: { testid: 'hidden-review-count' }) do
          m3_text(variant: :body_medium, class: 'font-medium text-on-surface-variant') do
            t('medication_reviews.hidden_count', count: hidden_count)
          end
          m3_link(href: hidden_toggle_path, variant: :text, size: :sm) do
            t(show_hidden ? 'medication_reviews.hide_hidden' : 'medication_reviews.show_hidden')
          end
        end
      end

      def render_prompts
        if prompts.empty?
          render Components::Shared::EmptyState.new(
            title: t('medication_reviews.empty_title'),
            description: t('medication_reviews.empty_description')
          )
          return
        end

        div(class: 'space-y-10') do
          prompts.group_by(&:person).each do |person, person_prompts|
            section('aria-labelledby': "person-review-heading-#{person.id}") do
              m3_heading(level: 2, size: '5', id: "person-review-heading-#{person.id}", class: 'mb-4 font-bold') do
                person.name
              end
              div(class: 'grid gap-5') do
                person_prompts.each { |prompt| render PromptCard.new(prompt: prompt) }
              end
            end
          end
        end
      end

      def hidden_toggle_path
        medication_review_prompts_path(show_hidden: show_hidden ? nil : '1')
      end
    end
  end
end
