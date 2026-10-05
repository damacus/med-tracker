# frozen_string_literal: true

require 'rails_helper'

RSpec.describe I18n do
  let(:key_paths) do
    %w[
      title subtitle table.id table.carer table.patient table.access_level table.relationship table.status
      table.created_at table.expires_at table.revoked_at empty active inactive not_available
      pagination.showing pagination.to pagination.of pagination.results pagination.previous pagination.next
      pagination.label
    ].map { |path| path.split('.') }
  end

  it 'keeps the queue translations structurally present in every locale' do
    locale_files.each do |locale_file|
      tree = YAML.safe_load(locale_file.read).fetch(locale_file.basename('.yml').to_s)
      key_paths.each do |key_path|
        message = "#{locale_file} is missing " \
                  "admin.ambiguous_person_access_grants.index.#{key_path.join('.')}"
        expect(tree.dig('admin', 'ambiguous_person_access_grants', 'index', *key_path)).to be_present, message
      end
    end
  end

  def locale_files
    Rails.root.glob('config/locales/*.yml')
  end
end
