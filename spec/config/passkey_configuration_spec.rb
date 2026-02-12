# frozen_string_literal: true

require 'rails_helper'

# rubocop:disable RSpec/DescribeClass
RSpec.describe 'PASSKEY-001: WebAuthn/Passkey configuration' do
  fixtures :accounts
  scenario 'WebAuthn key timestamps default to current timestamp' do
    columns = ActiveRecord::Base.connection.columns('account_webauthn_keys')
    defaults = columns.index_by(&:name).transform_values(&:default_function)

    expect(defaults['created_at']).to eq('CURRENT_TIMESTAMP')
    expect(defaults['updated_at']).to eq('CURRENT_TIMESTAMP')
  end

  scenario 'WebAuthn gem is available' do
    expect { WebAuthn }.not_to raise_error
  end

  scenario 'WebAuthn database tables exist' do
    # Check that tables were created
    expect(ActiveRecord::Base.connection.table_exists?('account_webauthn_keys')).to be true
    expect(ActiveRecord::Base.connection.table_exists?('account_webauthn_user_ids')).to be true

    # Check indexes
    indexes = ActiveRecord::Base.connection.indexes('account_webauthn_keys')
    expect(indexes.map(&:name)).to include('index_account_webauthn_keys_on_account_id')
    expect(indexes.map(&:name)).to include('index_account_webauthn_keys_on_webauthn_id_and_account_id')
  end

  scenario 'WebAuthn models are defined' do
    expect { AccountWebauthnKey }.not_to raise_error
    expect { AccountWebauthnUserId }.not_to raise_error
  end

  scenario 'Account model has WebAuthn associations' do
    account = accounts(:damacus)
    expect(account.respond_to?(:account_webauthn_keys)).to be true
    expect(account.respond_to?(:account_webauthn_user_ids)).to be true
  end

  scenario 'Rodauth WebAuthn feature is enabled' do
    # Verify Rodauth has WebAuthn configured by checking for WebAuthn-specific methods
    # The presence of these methods indicates the feature is enabled
    expect(RodauthMain.instance_methods).to include(:webauthn_setup_path)
    expect(RodauthMain.instance_methods).to include(:webauthn_auth_path)
  end
end
# rubocop:enable RSpec/DescribeClass
