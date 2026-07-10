# frozen_string_literal: true

module OpenFda
  class SnapshotManifest
    DEFAULT_PATH = Rails.root.join('config/medication_review_labels.yml')

    def initialize(path: DEFAULT_PATH)
      @data = YAML.safe_load_file(path)
    end

    def version
      data.fetch('version')
    end

    def source
      data.fetch('source')
    end

    def selection
      data.fetch('selection')
    end

    private

    attr_reader :data
  end
end
