# frozen_string_literal: true

module OpenFda
  class SnapshotClient
    DEFAULT_PATH = Rails.root.join('data/medication_reviews/openfda_labels.json')

    def initialize(path: DEFAULT_PATH)
      @path = path
    end

    def labels(limit:)
      available_labels = snapshot.fetch('labels')
      requested_limit = Integer(limit)
      if requested_limit > available_labels.size
        raise ArgumentError, "snapshot contains #{available_labels.size} labels; requested #{requested_limit}"
      end

      available_labels.first(requested_limit)
    end

    private

    attr_reader :path

    def snapshot
      @snapshot ||= JSON.parse(File.read(path))
    end
  end
end
