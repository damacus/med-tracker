# frozen_string_literal: true

module Nlm
  class RxClassSnapshotBuilder
    def initialize(client: RxClassClient.new, manifest: OpenFda::SnapshotManifest.new, generated_on: Date.current)
      @client = client
      @manifest = manifest
      @generated_on = generated_on
    end

    def call
      {
        'source' => 'NLM RxClass API using DailyMed relationships',
        'generated_on' => generated_on.iso8601,
        'selection_version' => manifest.version,
        'entries' => client.entries_for(manifest.selection)
      }
    end

    private

    attr_reader :client, :manifest, :generated_on
  end
end
