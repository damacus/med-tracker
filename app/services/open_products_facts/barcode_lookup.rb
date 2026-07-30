# frozen_string_literal: true

module OpenProductsFacts
  class BarcodeLookup
    def initialize(client: Client.new, audit_logger: ExternalLookup::AuditLogger.new)
      @client = client
      @audit_logger = audit_logger
    end

    def lookup(barcode)
      product = @client.product(barcode)
      return not_found(barcode) unless product

      process_product(barcode, product)
    rescue Client::ApiError => e
      Observability::DiagnosticEvent.failure(component: :open_products_facts, error: e, severity: :warn)
      audit(barcode, 'error')
      nil
    rescue StandardError => e
      Observability::DiagnosticEvent.failure(component: :open_products_facts, error: e)
      audit(barcode, 'error')
      nil
    end

    private

    def process_product(barcode, product)
      entry_attrs = ResultBuilder.catalog_entry_from_product(barcode, product)
      return not_found(barcode) unless entry_attrs

      persist(entry_attrs)
      audit(barcode, 'success', 1)
      entry_attrs
    end

    def not_found(barcode)
      audit(barcode, 'not_found')
      nil
    end

    # Persists the result so subsequent scans are served from the local catalogue
    # without hitting the API again.
    def persist(attrs)
      BarcodeCatalogEntry.find_or_create_by!(gtin: attrs[:gtin], source: attrs[:source]) do |entry|
        entry.display = attrs[:display]
        entry.system = attrs[:system]
        entry.concept_class = attrs[:concept_class]
      end
    rescue ActiveRecord::RecordInvalid, ActiveRecord::RecordNotUnique => e
      Observability::DiagnosticEvent.failure(component: :open_products_facts, error: e, severity: :warn)
    end

    def audit(barcode, status, count = 0)
      @audit_logger.record(source: 'open_products_facts', event: 'barcode_lookup',
                           query: barcode, result_status: status, result_count: count)
    end
  end
end
