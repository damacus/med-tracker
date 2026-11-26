# frozen_string_literal: true

module Shared
  class Navbar < Bridgetown::Component
    attr_reader :metadata

    def initialize(metadata:, resource:)
      @metadata = metadata
    end
  end

  class Sidebar < Bridgetown::Component
    def initialize(metadata:, resource:)
      @resource = resource
      @site = resource.site
    end

    def current?(path)
      @resource.data.source_path == path
    end
  end
end
