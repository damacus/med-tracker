module Components
  module Reports
    class ExportPanel < Components::Base
      def initialize(href:, fallback_href:, label:, preparing_label:, **content)
        @href = href
        @fallback_href = fallback_href
        @title = content.fetch(:title)
        @description = content.fetch(:description)
        @scope = content.fetch(:scope)
        @label = label
        @preparing_label = preparing_label
        super()
      end

      def view_template
        section(
          class: [
            'flex flex-col gap-4 rounded-[1.5rem] border border-border/70 bg-surface-container-low p-5',
            'sm:flex-row sm:items-center sm:justify-between'
          ].join(' '),
          data: { testid: 'pdf-export-panel' }
        ) do
          div(class: 'min-w-0 space-y-1') do
            m3_heading(level: 2, size: '3', class: 'font-bold') { @title }
            m3_text(size: '2', class: 'block text-on-surface-variant') { @description }
            m3_text(size: '1', class: 'block text-on-surface-variant') { @scope }
          end

          m3_link(
            href: @href,
            variant: :filled,
            class: 'shrink-0 gap-2 self-start sm:self-auto',
            aria: { busy: 'false', disabled: 'false' },
            data: {
              controller: 'pdf-export',
              action: 'click->pdf-export#prepare',
              pdf_export_fallback_location_value: @fallback_href,
              pdf_export_ready_label_value: @label,
              pdf_export_preparing_label_value: @preparing_label
            }
          ) do
            render Icons::FileText.new(size: 18, aria_hidden: 'true')
            span(data: { pdf_export_target: 'label' }, aria: { live: 'polite' }) { @label }
          end
        end
      end
    end
  end
end
