# frozen_string_literal: true

require 'rails_helper'

RSpec.describe I18n do
  let(:key_paths) do
    %w[
      index.title index.subtitle index.table.id index.table.carer index.table.patient index.table.access_level
      index.table.relationship index.table.status index.table.created_at index.table.expires_at
      index.table.revoked_at index.table.actions index.actions.reason index.actions.reason_placeholder
      index.actions.attach index.actions.manual index.actions.revoke index.empty index.active index.inactive
      index.not_available index.pagination.showing index.pagination.to index.pagination.of
      index.pagination.results index.pagination.previous index.pagination.next index.pagination.label
      classify.mfa_required classify.attach_notice classify.manual_notice classify.revoke_notice
    ].map { |path| path.split('.') }
  end

  it 'keeps the queue translations structurally present in every locale' do
    locale_files.each do |locale_file|
      tree = YAML.safe_load(locale_file.read).fetch(locale_file.basename('.yml').to_s)
      key_paths.each do |key_path|
        message = "#{locale_file} is missing " \
                  "admin.ambiguous_person_access_grants.#{key_path.join('.')}"
        expect(tree.dig('admin', 'ambiguous_person_access_grants', *key_path)).to be_present, message
      end
    end
  end

  def locale_files
    Rails.root.glob('config/locales/*.yml')
  end
end
