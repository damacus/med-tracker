# frozen_string_literal: true

require 'rails_helper'
require 'rake'

RSpec.describe HostedRestore::RuntimeVerifier do
  context 'with the hosted restore Rake tasks' do
    before { Rails.application.load_tasks unless Rake::Task.task_defined?('hosted_restore:verify_runtime') }

    around do |example|
      ENV['HOUSEHOLD_A_ID'] = '101'
      ENV['HOUSEHOLD_B_ID'] = '202'
      example.run
    ensure
      ENV.delete('HOUSEHOLD_A_ID')
      ENV.delete('HOUSEHOLD_B_ID')
      ENV.delete('WORM_HEADS_JSON')
      FileUtils.rm_f(Rails.root.join('tmp/runtime-image-ref'))
    end

    it 'emits sanitized runtime verification JSON' do
      result = { schema_version: '20260714090000', database_role: 'med_tracker_app', forced_rls: true,
                 app_image: 'app:v1',
                 default_deny: true, isolation: { clinical: true, audit: true, attachments: true },
                 storage: { samples_verified: 2 } }
      verifier = instance_double(described_class, call: result)
      allow(described_class).to receive(:new).and_return(verifier)

      expect { reenabled_task('hosted_restore:verify_runtime').invoke }
        .to output(/"outcome":"passed".*"database_role":"med_tracker_app"/).to_stdout
    end

    it 'checks the owner role before running migrations and reporting the schema' do
      order = stub_successful_owner_migration

      expect { reenabled_task('hosted_restore:migrate').execute }
        .to output(
          /"outcome":"passed","database_role":"med_tracker_owner","schema_version":"20260714090000"/
        ).to_stdout
      expect(order).to eq(%i[role reenable migration schema])
    end

    it 'never invokes migrations when the database role is not the owner' do
      connection = instance_double(ActiveRecord::ConnectionAdapters::PostgreSQLAdapter)
      allow(connection).to receive(:select_value).with('SELECT current_user').and_return('med_tracker_app')
      allow(ActiveRecord::Base).to receive(:connection).and_return(connection)
      migration_task = instance_double(Rake::Task, reenable: nil, invoke: nil)
      allow(Rake::Task).to receive(:[]).and_call_original
      allow(Rake::Task).to receive(:[]).with('db:migrate').and_return(migration_task)

      expect { reenabled_task('hosted_restore:migrate').execute }.to raise_error(SystemExit)
        .and output(/"failure_code":"migration_owner_role_required"/).to_stderr
      expect(migration_task).not_to have_received(:invoke)
    end

    it 'runs the real verifier with transaction-scoped tenant settings at the Rake boundary' do
      connection = HostedRestoreBoundaryFakes::Connection.new
      models = {
        clinical: HostedRestoreBoundaryFakes::Model.new(connection, 101 => 1, 202 => 2),
        audit: HostedRestoreBoundaryFakes::Model.new(connection, 101 => 3, 202 => 4),
        attachments: HostedRestoreBoundaryFakes::Model.new(connection, 101 => 5, 202 => 6)
      }
      verifier = described_class.new(
        household_ids: [101, 202], connection:, models:, runtime_image_path: runtime_image_path,
        storage_verifier: class_double(Storage::RestoreVerifier, call: true)
      )
      allow(described_class).to receive(:new).and_return(verifier)

      expect { reenabled_task('hosted_restore:verify_runtime').invoke }
        .to output(/"outcome":"passed".*"database_role":"med_tracker_app"/).to_stdout
      expect(connection.transaction_calls).to be >= 5
    end

    it 'exits nonzero with a sanitized failure code' do
      allow(described_class).to receive(:new)
        .and_raise(HostedRestore::VerificationError, 'runtime_role_required')

      expect { reenabled_task('hosted_restore:verify_runtime').invoke }.to raise_error(SystemExit)
        .and output(/"outcome":"failed","failure_code":"runtime_role_required"/).to_stderr
    end

    it 'emits only sanitized aggregate audit and WORM comparison evidence' do
      ENV['WORM_HEADS_JSON'] = '{"sample_a":{},"sample_b":{}}'
      result = { scope: 'combined', samples_verified: 2, checked_entries: 8, checked_checkpoints: 2,
                 checked_objects: 10, verified_heads: 2, worm_comparison: 'match' }
      verifier = instance_double(HostedRestore::AuditVerifier, call: result)
      allow(HostedRestore::AuditVerifier).to receive(:new).and_return(verifier)

      expect { reenabled_task('hosted_restore:verify_audit').invoke }
        .to output(/"outcome":"passed".*"worm_comparison":"match"/).to_stdout
    end

    def reenabled_task(name)
      Rake::Task[name].tap(&:reenable)
    end

    def stub_successful_owner_migration
      order = []
      allow(ActiveRecord::Base).to receive(:connection).and_return(owner_connection(order))
      allow(Rake::Task).to receive(:[]).and_call_original
      allow(Rake::Task).to receive(:[]).with('db:migrate').and_return(migration_task(order))
      order
    end

    def owner_connection(order)
      instance_double(ActiveRecord::ConnectionAdapters::PostgreSQLAdapter).tap do |connection|
        allow(connection).to receive(:select_value).with('SELECT current_user') do
          order << :role
          'med_tracker_owner'
        end
        allow(connection).to receive(:select_value).with('SELECT max(version) FROM schema_migrations') do
          order << :schema
          '20260714090000'
        end
      end
    end

    def migration_task(order)
      instance_double(Rake::Task).tap do |task|
        allow(task).to receive(:reenable) { order << :reenable }
        allow(task).to receive(:invoke) { order << :migration }
      end
    end

    def runtime_image_path
      Rails.root.join('tmp/runtime-image-ref').tap { |path| path.write("app:v1\n") }
    end
  end
end
