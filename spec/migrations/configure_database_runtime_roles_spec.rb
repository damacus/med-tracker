# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'ConfigureDatabaseRuntimeRoles' do
  it 'raises a pre-0.5 bootstrap error when runtime roles are missing and cannot be created' do
    migration = ConfigureDatabaseRuntimeRoles.new

    allow(migration).to receive_messages(runtime_role_exists?: false, can_create_roles?: false)

    expect { migration.send(:ensure_runtime_roles_bootstrapped!) }
      .to raise_error(ActiveRecord::IrreversibleMigration, /pre-0.5 database upgrade/)
  end
end
