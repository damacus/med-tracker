# frozen_string_literal: true

module Components
  module Medications
    class NhsGuidanceFrame < Components::Base
      include Phlex::Rails::Helpers::TurboFrameTag

      attr_reader :medication, :guidance, :src

      def initialize(medication:, guidance: nil, src: nil)
        @medication = medication
        @guidance = guidance
        @src = src
        super()
      end

      def view_template
        turbo_frame_tag(frame_id, src: src, loading: :lazy, class: 'block') do
          render_guidance_section if guidance.present?
        end
      end

      private

      def frame_id
        "medication_#{medication.id}_nhs_guidance"
      end

      def render_guidance_section
        div(class: 'space-y-4') do
          m3_heading(variant: :title_large, level: 2, class: 'font-bold tracking-tight') do
            t('medications.show.nhs_guidance_title')
          end
          m3_card(variant: :elevated, class: 'border border-border/60 p-8 space-y-6') do
            render_guidance_header
            render_guidance_sections
            render_guidance_footer
          end
        end
      end

      def render_guidance_header
        div(class: 'flex flex-col gap-4 md:flex-row md:items-start md:justify-between') do
          div(class: 'space-y-2') do
            m3_heading(variant: :title_medium, level: 3, class: 'font-bold') { guidance.title }
            m3_text(variant: :body_large, class: 'text-on-surface-variant leading-relaxed font-medium') do
              guidance.description
            end
            render_review_date if guidance.last_reviewed_on.present?
          end

          if guidance_author_logo.present?
            a(href: guidance.webpage, target: '_blank', rel: 'noopener', class: 'shrink-0') do
              img(src: guidance_author_logo, alt: guidance_author_name, class: 'h-10 w-auto')
            end
          end
        end
      end

      def render_review_date
        m3_text(variant: :label_medium, class: 'text-on-surface-variant') do
          t('medications.show.nhs_guidance_reviewed_on',
            date: I18n.l(guidance.last_reviewed_on, format: :long))
        end
      end

      def render_guidance_sections
        return if guidance.sections.empty?

        div(class: 'grid gap-4') do
          guidance.sections.each do |section|
            div(class: 'rounded-shape-xl bg-surface-container-low p-4 space-y-2') do
              m3_heading(variant: :title_small, level: 4, class: 'font-bold') { section.title }
              m3_text(variant: :body_medium, class: 'text-on-surface-variant leading-relaxed') do
                section.text
              end
            end
          end
        end
      end

      def render_guidance_footer
        div(class: 'flex flex-wrap items-center justify-between gap-4 pt-2') do
          render_guidance_source if guidance_author_name.present?

          m3_link(
            href: guidance.webpage,
            target: '_blank',
            rel: 'noopener',
            variant: :outlined,
            size: :md
          ) do
            t('medications.show.nhs_guidance_link')
          end
        end
      end

      def render_guidance_source
        if guidance_author_url.present?
          m3_text(variant: :label_medium, class: 'text-on-surface-variant') do
            a(href: guidance_author_url, target: '_blank', rel: 'noopener',
              class: 'underline decoration-border underline-offset-4') do
              plain t('medications.show.nhs_guidance_source', source: guidance_author_name)
            end
          end
        else
          m3_text(variant: :label_medium, class: 'text-on-surface-variant') do
            plain t('medications.show.nhs_guidance_source', source: guidance_author_name)
          end
        end
      end

      def guidance_author_logo
        guidance.author_logo if guidance.respond_to?(:author_logo)
      end

      def guidance_author_name
        guidance.author_name if guidance.respond_to?(:author_name)
      end

      def guidance_author_url
        guidance.author_url if guidance.respond_to?(:author_url)
      end
    end
  end
end
