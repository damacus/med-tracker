module Components
  module Reports
    class PdfDocument < Phlex::HTML
      def initialize(title:, context:, generated_at:, content:, locale: 'en')
        @title = title
        @context = context
        @generated_at = generated_at
        @content = content
        @locale = locale
        super()
      end

      def view_template
        doctype
        html(lang: @locale) do
          head do
            meta(charset: 'utf-8')
            title { @title }
            render PdfStylesheet.new
          end
          body do
            header(class: 'report-header') do
              p(class: 'report-brand') { 'MEDTRACKER' }
              h1 { @title }
              p(class: 'report-context') { @context }
              p(class: 'report-generated') do
                plain 'Generated '
                time(datetime: @generated_at.iso8601) do
                  @generated_at.utc.strftime('%-d %B %Y at %H:%M UTC')
                end
              end
            end
            main { render @content }
          end
        end
      end
    end
  end
end
