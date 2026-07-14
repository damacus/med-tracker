# frozen_string_literal: true

require 'rails_helper'

RSpec.describe HostedRestore::Rehearsal do
  context 'with the hosted restore documentation' do
    let(:runbook) { Rails.root.join('docs/operations/hosted-private-beta-runbook.md').read }
    let(:audit) { Rails.root.join('docs/security/hosted-multi-tenant-hardening-audit.md').read }

    it 'documents the executable gate, durable evidence contract, cadence, and invalidation triggers' do
      expect(runbook).to include(
        'task hosted-restore:rehearse', 'DATABASE_BACKUP_ID', 'ATTACHMENT_BACKUP_ID', 'APP_IMAGE',
        'RUNTIME_APP_IMAGE', 'TESTER', 'WORM_HEADS_JSON', 'EVIDENCE_ROOT', 'EVIDENCE_OUTPUT',
        'evidence.json', 'evidence.md',
        'at least quarterly', 'database major version', 'RLS policies', 'Object Lock'
      )
      expect(audit).to include('task hosted-restore:rehearse', 'no current production-like database and attachment')
      expect(audit).to include('[ ] A production-like backup restore rehearsal has passed')
      expect(audit).to include('| NFR4 |').and include('| NO-GO |')
    end

    it 'forbids fabricated, transient, sensitive, or tenant-identifying evidence' do
      expect(runbook.squish).to include(
        'does not accept or run a raw restore command', 'not the source tree or transient container filesystem',
        'real tenant identifiers', 'partial work can never claim success',
        'resolves by realpath below', 'runtime-provided immutable image reference'
      )
    end
  end
end
