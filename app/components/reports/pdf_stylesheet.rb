module Components
  module Reports
    class PdfStylesheet < Phlex::HTML
      def view_template
        style do
          raw safe(<<~CSS)
            @page {
              size: A4;
              margin: 18mm 16mm 20mm;
              @bottom-right { content: "Page " counter(page) " of " counter(pages); color: #5B6864; font-size: 8pt; }
            }

            * { box-sizing: border-box; }
            body { margin: 0; color: #17211F; background: #FFFFFF; font-family: "Noto Sans", sans-serif; font-size: 10pt; line-height: 1.5; }
            .report-header { margin: -18mm -16mm 10mm; padding: 12mm 16mm 10mm; background: #174A46; color: #FFFFFF; }
            .report-brand { font-size: 8pt; font-weight: 700; letter-spacing: 0.14em; }
            .report-header h1 { margin: 3mm 0 2mm; font-size: 22pt; line-height: 1.15; }
            .report-context, .report-generated { margin: 0; color: #DDEBE7; }
            .report-generated { font-size: 8pt; }
            .report-section { margin: 0 0 7mm; }
            h2, h3 { color: #174A46; break-after: avoid; page-break-after: avoid; }
            h2 { margin: 0 0 3mm; font-size: 15pt; }
            h3 { margin: 0 0 2mm; font-size: 11pt; }
            p, li { widows: 3; orphans: 3; }
            .callout { margin: 4mm 0; padding: 4mm; border: 0.4mm solid #174A46; background: #DDEBE7; break-inside: avoid; page-break-inside: avoid; }
            .empty-state { margin: 12mm 0; padding: 8mm; border: 0.3mm solid #D7DEDB; background: #F7F9F8; color: #5B6864; text-align: center; break-inside: avoid; page-break-inside: avoid; }
            table { width: 100%; border-collapse: collapse; margin: 4mm 0; }
            thead { display: table-header-group; background: #F7F9F8; color: #174A46; }
            tr { break-inside: avoid; page-break-inside: avoid; }
            th, td { padding: 2.5mm; border: 0.3mm solid #D7DEDB; text-align: left; vertical-align: top; }
            th { font-size: 8pt; letter-spacing: 0.04em; text-transform: uppercase; }
          CSS
        end
      end
    end
  end
end
