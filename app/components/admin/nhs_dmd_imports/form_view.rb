# frozen_string_literal: true

module Components
  module Admin
    module NhsDmdImports
      class FormView < Components::Base
        include Phlex::Rails::Helpers::FormWith

        def initialize(import_run: nil)
          @import_run = import_run
          super()
        end

        def view_template
          div(class: 'container mx-auto px-4 py-12 max-w-3xl space-y-8') do
            render_header
            render_form
            render_import_run
            render_auto_refresh
          end
        end

        private

        def render_header
          div(class: 'space-y-2') do
            m3_heading(variant: :display_small, level: 1, class: 'font-black tracking-tight') do
              t('admin.nhs_dmd_imports.title')
            end
            m3_text(variant: :body_large, class: 'text-on-surface-variant font-medium') do
              t('admin.nhs_dmd_imports.subtitle')
            end
          end
        end

        def render_form
          form_with(url: admin_nhs_dmd_import_path, scope: :nhs_dmd_import, multipart: true, class: 'space-y-6') do
            m3_card(variant: :elevated, class: 'border border-border/60 p-8 space-y-6') do
              div(class: 'space-y-2') do
                render RubyUI::FormFieldLabel.new(
                  for: 'nhs_dmd_import_release_zip',
                  class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1'
                ) { t('admin.nhs_dmd_imports.form.release_zip') }
                input(
                  type: 'file',
                  id: 'nhs_dmd_import_release_zip',
                  name: 'nhs_dmd_import[release_zip]',
                  accept: '.zip,application/zip',
                  required: true,
                  class: 'block w-full rounded-md border border-outline-variant bg-surface-container-lowest px-4 py-4'
                )
                m3_text(variant: :body_medium, class: 'text-on-surface-variant') do
                  t('admin.nhs_dmd_imports.form.help')
                end
              end

              div(class: 'flex flex-wrap gap-3') do
                button(
                  type: 'submit',
                  class: 'inline-flex items-center justify-center rounded-shape-full bg-primary px-5 py-3 ' \
                         'font-bold text-on-primary transition hover:opacity-90'
                ) do
                  t('admin.nhs_dmd_imports.form.submit')
                end
                m3_link(href: '/admin', variant: :text, size: :lg, class: 'font-bold') do
                  t('admin.nhs_dmd_imports.form.cancel')
                end
              end
            end
          end
        end

        def render_import_run
          return unless @import_run

          m3_card(variant: :elevated, class: 'border border-border/60 p-8 space-y-5') do
            div(class: 'flex flex-wrap items-start justify-between gap-3') do
              div(class: 'space-y-1') do
                m3_heading(level: 2, size: '4', class: 'font-bold') { t('admin.nhs_dmd_imports.latest_run.title') }
                m3_text(variant: :body_medium, class: 'text-on-surface-variant') { @import_run.uploaded_filename }
              end
              span(
                class: 'rounded-full bg-secondary-container px-3 py-1 text-xs font-black uppercase tracking-wider'
              ) do
                t("admin.nhs_dmd_imports.statuses.#{@import_run.status}")
              end
            end

            div(class: 'grid gap-4 sm:grid-cols-2') do
              render_stat(t('admin.nhs_dmd_imports.latest_run.progress'), progress_text)
              render_stat(t('admin.nhs_dmd_imports.latest_run.percentage'), percentage_text)
              render_stat(t('admin.nhs_dmd_imports.latest_run.imported'), @import_run.imported_count)
              render_stat(t('admin.nhs_dmd_imports.latest_run.skipped'), @import_run.skipped_count)
            end

            if @import_run.error_message.present?
              div(class: 'rounded-xl border border-destructive/40 bg-destructive/10 p-4 text-sm text-destructive') do
                @import_run.error_message
              end
            end

            div(class: 'space-y-2') do
              m3_text(variant: :label_large, class: 'font-black uppercase tracking-widest text-on-surface-variant') do
                t('admin.nhs_dmd_imports.latest_run.log')
              end
              pre(
                class: 'max-h-60 overflow-y-auto rounded-xl border border-border bg-surface-container-lowest ' \
                       'p-4 text-sm whitespace-pre-wrap'
              ) do
                plain @import_run.log.presence || t('admin.nhs_dmd_imports.latest_run.no_log')
              end
            end
          end
        end

        def render_stat(label, value)
          div(class: 'rounded-xl border border-border/70 bg-surface-container-lowest p-4') do
            m3_text(
              variant: :label_medium,
              class: 'uppercase tracking-widest text-on-surface-variant font-black'
            ) { label }
            m3_heading(level: 3, size: '4', class: 'font-bold mt-2') { value.to_s }
          end
        end

        def progress_text
          return t('admin.nhs_dmd_imports.latest_run.awaiting_count') unless @import_run.total_records.positive?

          "#{@import_run.processed_records} / #{@import_run.total_records}"
        end

        def percentage_text
          return '0%' unless @import_run.total_records.positive?

          "#{@import_run.progress_percentage}%"
        end

        def render_auto_refresh
          return unless @import_run&.active?

          script do
            plain 'window.setTimeout(function () { window.location.reload(); }, 5000);'
          end
        end
      end
    end
  end
end
