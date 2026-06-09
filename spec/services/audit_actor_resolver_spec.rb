# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AuditActorResolver do
  fixtures :accounts, :people, :users

  subject(:resolver) { described_class.new }

  it 'returns the system label for a blank whodunnit' do
    expect(resolver.name_for(nil)).to eq(I18n.t('admin.audit_logs.index.system'))
    expect(resolver.name_for('')).to eq(I18n.t('admin.audit_logs.index.system'))
  end

  it 'returns the user name for a known id' do
    admin = users(:admin)

    expect(resolver.name_for(admin.id.to_s)).to eq(admin.name)
  end

  it 'returns a fallback label for an unknown id' do
    expect(resolver.name_for('999999')).to eq('User #999999')
  end

  it 'caches lookups so repeated ids hit the database once' do
    admin = users(:admin)

    allow(User).to receive(:find_by).and_call_original

    2.times { resolver.name_for(admin.id.to_s) }

    expect(User).to have_received(:find_by).once
  end
end
