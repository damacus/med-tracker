# frozen_string_literal: true

module Components
  module Medications
    # rubocop:disable Layout/LineLength
    class FinderView < Components::Base
      def view_template
        div(
          data: {
            testid: 'medication-finder',
            controller: 'medication-search',
            action: 'barcode-scanner:decoded->medication-search#barcodeDecoded',
            medication_search_translations_value: {
              loading: t('medications.finder.loading'),
              idleText: t('medications.finder.idle_text'),
              unavailableTitle: t('medications.finder.unavailable_title'),
              unavailableMessage: t('medications.finder.unavailable_message'),
              noResultsTitle: t('medications.finder.no_results_title'),
              noResultsMessage: t('medications.finder.no_results_message'),
              resultsTitle: t('medications.finder.results_title'),
              source: t('medications.finder.source'),
              dmdCode: t('medications.finder.dmd_code'),
              resultCount: {
                one: t('medications.finder.result_count.one'),
                other: t('medications.finder.result_count.other')
              }
            }.to_json
          },
          class: 'container mx-auto px-4 py-12 max-w-5xl'
        ) do
          render_header
          render_scanner_section
          render_search_section
          render_results_section
        end
      end

      private

      def render_header
        div(class: 'mb-10') do
          Text(size: '2', weight: 'muted', class: 'uppercase tracking-widest mb-1 block font-bold') { t('medications.finder.nhs_directory') }
          Heading(level: 1, size: '7', class: 'font-bold tracking-tight') { t('medications.finder.title') }
          Text(size: '3', class: 'text-muted-foreground mt-2') do
            plain t('medications.finder.subtitle')
          end
        end
      end

      def render_scanner_section
        div(class: 'mb-8') do
          render Components::BarcodeScanner.new
        end
      end

      def render_search_section
        form(
          data: { action: 'submit->medication-search#search', medication_search_target: 'form' },
          class: 'mb-12'
        ) do
          div(class: 'relative group') do
            div(class: 'absolute inset-y-0 left-0 pl-5 flex items-center pointer-events-none text-muted-foreground group-focus-within:text-primary transition-colors') do
              render Icons::Search.new(size: 22)
            end
            input(
              type: 'text',
              id: 'medication-search-input',
              name: 'q',
              placeholder: t('medications.finder.placeholder'),
              autocomplete: 'off',
              data: { medication_search_target: 'input' },
              class: 'block w-full pl-14 pr-32 py-6 border border-border rounded-[1.5rem] leading-5 bg-cardest shadow-[0_10px_40px_rgba(0,0,0,0.03)] focus:shadow-[0_10px_40px_rgba(0,0,0,0.06)] focus:outline-none focus:ring-4 focus:ring-primary/5 focus:border-primary sm:text-base transition-all placeholder:text-muted-foreground'
            )
            div(class: 'absolute inset-y-2 right-2 flex items-center') do
              Button(
                type: :submit,
                variant: :primary,
                class: 'h-full rounded-shape-xl px-8 font-bold text-sm shadow-lg shadow-primary/20',
                data: { medication_search_target: 'submitButton' }
              ) { t('medications.finder.search_button') }
            end
          end
        end
      end

      def render_results_section
        div(data: { medication_search_target: 'results' }, class: 'space-y-6') do
          render_idle_state
        end
      end

      def render_idle_state
        div(data: { medication_search_target: 'idle' }, class: 'text-center py-24 px-8 rounded-[3rem] border-2 border-dashed border-border') do
          div(class: 'w-20 h-20 rounded-full bg-card flex items-center justify-center text-muted-foreground mx-auto mb-6') do
            render Icons::Search.new(size: 40)
          end
          Heading(level: 3, size: '5', class: 'font-bold text-muted-foreground mb-2') { t('medications.finder.idle_heading') }
          Text(size: '2', class: 'text-muted-foreground max-w-sm mx-auto') do
            t('medications.finder.idle_text')
          end
        end
      end
    end
    # rubocop:enable Layout/LineLength
  end
end
