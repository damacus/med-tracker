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
            body { margin: 0; color: #17211F; background: #FFFFFF; font-family: "Noto Sans", sans-serif; font-size: 10pt; line-height: 1.5; font-variant-ligatures: none; }
            .report-header { margin: 0 -16mm 10mm; padding: 12mm 16mm 10mm; background: #174A46; color: #FFFFFF; }
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
            .report-summary { display: table; width: 100%; background: #F7F9F8; }
            .summary-item { display: table-cell; width: 33%; padding: 4mm; text-align: center; }
            .summary-item strong, .summary-item span { display: block; }
            .summary-item strong { color: #174A46; font-size: 16pt; }
            .summary-item span, .person-review-count, .review-status { color: #5B6864; font-size: 8pt; text-transform: uppercase; }
            .review-prompt { padding: 4mm; border: 0.3mm solid #D7DEDB; break-inside: avoid; page-break-inside: avoid; }
            .review-prompt-heading { margin-bottom: 3mm; }
            .review-prompt-heading h3 { margin-bottom: 1mm; }
            .risk-legend { margin: 0; padding-left: 5mm; }
            .parenthetical::before { content: "("; }
            .parenthetical::after { content: ")"; }
            .risk-high { color: #B94B43; }
            .risk-moderate { color: #B27718; }
            .risk-low { color: #4E725A; }
            .risk-unknown { color: #6B7280; }
            table { width: 100%; border-collapse: collapse; table-layout: fixed; margin: 4mm 0; }
            .health-history-side-effects-table th:nth-child(1), .health-history-side-effects-table td:nth-child(1) { width: 8%; }
            .health-history-side-effects-table th:nth-child(2), .health-history-side-effects-table td:nth-child(2) { width: 14%; }
            .health-history-side-effects-table th:nth-child(3), .health-history-side-effects-table td:nth-child(3) { width: 10%; }
            .health-history-side-effects-table th:nth-child(4), .health-history-side-effects-table td:nth-child(4) { width: 15%; }
            .health-history-side-effects-table th:nth-child(5), .health-history-side-effects-table td:nth-child(5) { width: 40%; }
            .health-history-side-effects-table th:nth-child(6), .health-history-side-effects-table td:nth-child(6) { width: 13%; }
            .health-history-illnesses-table th:nth-child(1), .health-history-illnesses-table td:nth-child(1) { width: 12%; }
            .health-history-illnesses-table th:nth-child(2), .health-history-illnesses-table td:nth-child(2) { width: 18%; }
            .health-history-illnesses-table th:nth-child(3), .health-history-illnesses-table td:nth-child(3) { width: 12%; }
            .health-history-illnesses-table th:nth-child(4), .health-history-illnesses-table td:nth-child(4) { width: 42%; }
            .health-history-illnesses-table th:nth-child(5), .health-history-illnesses-table td:nth-child(5) { width: 16%; }
            thead { display: table-header-group; background: #F7F9F8; color: #174A46; }
            tr { break-inside: avoid; page-break-inside: avoid; }
            th, td { padding: 2.5mm; border: 0.3mm solid #D7DEDB; overflow-wrap: anywhere; text-align: left; vertical-align: top; }
            th { font-size: 8pt; letter-spacing: 0.04em; text-transform: uppercase; }
          CSS
        end
      end
    end
  end
end
