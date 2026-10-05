# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'AllowAccountLinkedPersonRlsLoginLookup' do
  before do
    unless defined?(AllowAccountLinkedPersonRlsLoginLookup)
      load Rails.root.join('db/migrate/20260630141000_allow_account_linked_person_rls_login_lookup.rb')
    end
  end

  it 'does not create the login lookup policy before the runtime role exists' do
    migration = AllowAccountLinkedPersonRlsLoginLookup.new

    allow(migration).to receive_messages(table_exists?: true, column_exists?: true, runtime_role_exists?: false)
    allow(migration).to receive(:execute)

    expect { migration.up }.not_to raise_error
    expect(migration).not_to have_received(:execute)
  end
end
