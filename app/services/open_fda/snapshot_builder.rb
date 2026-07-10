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
      targeted_response = client.labels_for_targeted(manifest.targeted_selection)
      {
        'selection_version' => manifest.version,
        'generated_on' => generated_on.iso8601,
        'openfda_last_updated' => response.dig('meta', 'last_updated'),
        'labels' => foundation_labels(response) + targeted_labels(targeted_response)
      }
    end

    private

    attr_reader :client, :manifest, :generated_on

    def foundation_labels(response)
      response.fetch('results').zip(manifest.selection).map do |label, selection_term|
        snapshot_label(label, selection_term)
      end
    end

    def targeted_labels(response)
      response.fetch('results').zip(manifest.targeted_selection).map do |label, entry|
        snapshot_label(label, entry.fetch('term'), interaction_targets: entry.fetch('interaction_targets'))
      end
    end

    def snapshot_label(label, selection_term, interaction_targets: nil)
      snapshot = label.slice(*LABEL_FIELDS).merge(
        'selection_term' => selection_term,
        'openfda' => label.fetch('openfda', {}).slice(*OPENFDA_FIELDS)
      )
      snapshot['interaction_targets'] = interaction_targets if interaction_targets
      snapshot
    end
  end
end
