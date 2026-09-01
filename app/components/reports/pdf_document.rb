module Components
  module Reports
    class PdfDocument < Phlex::HTML
      def initialize(title:, context:, generated_at:, content:, **options)
        @title = title
        @context = context
        @generated_at = generated_at
        @content = content
        @locale = options.fetch(:locale, 'en')
        @page_number = options.fetch(:page_number, nil)
        @generated_at_text = options.fetch(:generated_at_text, nil)
        @context_lines = options.fetch(:context_lines, nil)
        super()
      end

      def view_template
        doctype
        html(lang: @locale) do
          head do
            meta(charset: 'utf-8')
            title { @title }
            render PdfStylesheet.new(page_number: @page_number)
          end
          body do
            header(class: 'report-header') do
              p(class: 'report-brand') { 'MEDTRACKER' }
              h1 { @title }
              p(class: 'report-context') do
                if @context_lines
                  @context_lines.each_with_index do |line, index|
                    plain line
                    br unless index == @context_lines.length - 1
                  end
                else
                  @context
                end
              end
              p(class: 'report-generated') do
                if @generated_at_text
                  @generated_at_text
                else
                  plain 'Generated '
                  time(datetime: @generated_at.iso8601) do
                    @generated_at.utc.strftime('%-d %B %Y at %H:%M UTC')
                  end
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
