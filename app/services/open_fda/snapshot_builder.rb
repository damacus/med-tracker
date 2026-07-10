# frozen_string_literal: true

module OpenFda
  class SnapshotBuilder
    OPENFDA_FIELDS = %w[brand_name generic_name substance_name pharm_class_epc rxcui].freeze
    LABEL_FIELDS = %w[set_id id effective_time version drug_interactions].freeze

    def initialize(client: DrugLabelClient.new, manifest: SnapshotManifest.new, generated_on: Date.current)
      @client = client
      @manifest = manifest
      @generated_on = generated_on
    end

    def call
      response = client.labels_for(manifest.selection)
      {
        'selection_version' => manifest.version,
        'generated_on' => generated_on.iso8601,
        'openfda_last_updated' => response.dig('meta', 'last_updated'),
        'labels' => response.fetch('results').zip(manifest.selection).map do |label, selection_term|
          snapshot_label(label, selection_term)
        end
      }
    end

    private

    attr_reader :client, :manifest, :generated_on

    def snapshot_label(label, selection_term)
      label.slice(*LABEL_FIELDS).merge(
        'selection_term' => selection_term,
        'openfda' => label.fetch('openfda', {}).slice(*OPENFDA_FIELDS)
      )
    end
  end
end
