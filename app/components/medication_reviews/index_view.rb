# frozen_string_literal: true

module Components
  module MedicationReviews
    class IndexView < Components::Base
      def initialize(prompts:, filters:)
        @prompts = prompts
        @filters = filters
        super()
      end

      def view_template
        div(class: 'container mx-auto max-w-6xl px-4 py-10 pb-24 md:py-12', data: { testid: 'medicine-reviews' }) do
          render_header
          render Filters.new(**filters)
          render_prompts
        end
      end

      private

      attr_reader :prompts, :filters

      def render_header
        header(class: 'mb-8 flex flex-col gap-6 md:flex-row md:items-end md:justify-between') do
          div do
            m3_heading(level: 1, size: '8', class: 'font-black') { t('medication_reviews.title') }
            m3_text(variant: :body_large, class: 'mt-4 block max-w-3xl text-on-surface-variant') do
              t('medication_reviews.subtitle')
            end
            m3_text(variant: :body_medium, class: 'mt-1 block max-w-3xl text-on-surface-variant') do
              t('medication_reviews.boundary')
            end
          end
          m3_link(href: medication_review_report_path, variant: :filled, class: 'gap-2 self-start md:self-auto') do
            render Icons::FileText.new(size: 18)
            plain t('medication_reviews.export_pdf')
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
              div(class: 'grid gap-4') do
                person_prompts.each { |prompt| render PromptCard.new(prompt: prompt) }
              end
            end
          end
        end
      end
    end
  end
end
