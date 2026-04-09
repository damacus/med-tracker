# frozen_string_literal: true

module Components
  # rubocop:disable Layout/LineLength
  class BarcodeScanner < Components::Base
    def initialize(formats: nil, **attrs)
      @formats = formats || %w[EAN_13 EAN_8 CODE_128 CODE_39 QR_CODE]
      super(**attrs)
    end

    def view_template
      div(
        data: {
          testid: 'barcode-scanner',
          controller: 'barcode-scanner',
          barcode_scanner_formats_value: @formats.to_json,
          barcode_scanner_translations_value: {
            requesting: t('barcode_scanner.status.requesting'),
            scanning: t('barcode_scanner.status.scanning'),
            decoded: t('barcode_scanner.status.decoded'),
            denied: t('barcode_scanner.status.denied'),
            unavailable: t('barcode_scanner.status.unavailable'),
            error: t('barcode_scanner.status.error')
          }.to_json
        },
        class: 'rounded-2xl border border-border bg-surface-container-lowest p-6'
      ) do
        render_header
        render_scanner_region
        render_status
        render_manual_input
      end
    end

    private

    def render_header
      div(class: 'flex items-center justify-between mb-4') do
        div(class: 'flex items-center gap-2') do
          render Icons::Camera.new(size: 20)
          Heading(level: 3, size: '4', class: 'font-semibold') { t('barcode_scanner.title') }
        end
        render_control_buttons
      end
    end

    def render_control_buttons
      div(class: 'flex gap-2') do
        Button(
          variant: :outline,
          size: :sm,
          data: {
            barcode_scanner_target: 'startButton',
            action: 'barcode-scanner#start'
          },
          class: 'min-h-[44px] min-w-[44px]'
        ) { t('barcode_scanner.start') }

        Button(
          variant: :outline,
          size: :sm,
          data: {
            barcode_scanner_target: 'stopButton',
            action: 'barcode-scanner#stop'
          },
          hidden: true,
          class: 'min-h-[44px] min-w-[44px]'
        ) { t('barcode_scanner.stop') }
      end
    end

    def render_scanner_region
      div(
        id: 'barcode-scanner-region',
        data: { barcode_scanner_target: 'scannerRegion' },
        hidden: true,
        class: 'mb-4 rounded-xl overflow-hidden bg-black min-h-[200px]'
      )
    end

    def render_status
      p(
        data: { barcode_scanner_target: 'status' },
        class: 'text-sm text-muted-foreground mb-4 min-h-[1.25rem]',
        role: 'status',
        aria_live: 'polite'
      )
    end

    def render_manual_input
      div(
        class: 'border-t border-border pt-4',
        data: { testid: 'manual-barcode-input' }
      ) do
        label(for: 'manual-barcode', class: 'block text-sm font-medium text-foreground mb-1') do
          t('barcode_scanner.manual.label')
        end
        div(class: 'flex gap-2') do
          input(
            type: 'text',
            id: 'manual-barcode',
            placeholder: t('barcode_scanner.manual.placeholder'),
            data: { barcode_scanner_target: 'manualInput' },
            class: 'flex-1 rounded-lg border border-border px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary/20 focus:border-primary'
          )
          Button(
            variant: :outline,
            size: :sm,
            data: { action: 'barcode-scanner#submitManual' },
            class: 'min-h-[44px] min-w-[44px]'
          ) { t('barcode_scanner.manual.submit') }
        end
      end
    end
  end
  # rubocop:enable Layout/LineLength
end
