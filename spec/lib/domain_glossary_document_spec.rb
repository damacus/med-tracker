# frozen_string_literal: true

require 'rails_helper'

class DomainGlossaryDocument
  def glossary = Rails.root.join('docs/glossary.md').read

  def readme = Rails.root.join('README.md').read

  def operational_contract = Rails.root.join('docs/operations/record-lifecycle.md').read

  def context_map = Rails.root.join('docs/adrs/0009-bounded-context-map.md').read

  def docs_index = Rails.root.join('docs/index.md').read

  def navigation = Rails.root.join('zensical.toml').read
end

RSpec.describe DomainGlossaryDocument do
  subject(:document) { described_class.new }

  it 'defines the canonical ubiquitous language terms' do
    canonical_terms = [
      '### Person',
      '### Schedule',
      '### PersonMedication',
      '### MedicationTake',
      '### Supply',
      '### Carer'
    ]

    expect(document.glossary).to include(*canonical_terms)
  end

  it 'documents naming guidance and is discoverable from the README' do
    expect(document.glossary).to include('Prefer these names in new code, UI copy, documentation, and tests.')
    expect(document.glossary).to include(
      'Avoid introducing new patient, individual, plan, routine, stock, or dose-record'
    )
    expect(document.readme).to include('[Glossary](docs/glossary.md)')
  end

  it 'defines reversible cold-storage behaviour for medication, person, and location roots' do
    expect(document.operational_contract).to include('Retirement is explicit and reversible.')
    expect(document.operational_contract).to include('Logical cold storage')
    expect(document.operational_contract).to match(/Retiring a Medication\s+retires only that Medication/)
    expect(document.operational_contract).to match(
      /Person retirement never retires, deactivates, deletes, or otherwise changes\s+another Person/
    )
    expect(document.operational_contract).to match(
      /Location retirement is blocked while the location is primary or holds active\s+stock/
    )
  end

  it 'defines explicit root state machines and lifecycle safety boundaries' do
    expect(document.operational_contract).to include(
      'states `active`, `retired`, and `hard_deleted`',
      'The selected root itself changes lifecycle state',
      'Medication Catalogue owns the Medication root lifecycle',
      'HTTP 409 Conflict',
      '"code": "conflict"',
      '"message": "Record has changed since it was last read"'
    )
    expect(document.operational_contract).to match(
      /Medication Administration owns only the child\s+administration sources/
    )
  end

  it 'protects dependants when a carer is retired or deactivated' do
    expect(document.operational_contract).to include(
      'privacy-safe warning',
      'minor or dependent adult',
      'after explicit confirmation',
      'transition may proceed',
      'dependant remains active',
      'needs-carer workflow'
    )
    expect(document.operational_contract).to match(/does\s+not silently deactivate the dependant/)
    expect(document.operational_contract).to match(/Reactivation\s+never recreates care relationships/)
  end

  it 'makes reactivation non-cascading and preserves history and identity across exchange' do
    expect(document.operational_contract).to match(
      /Every `MedicationTake`, its source, and its root\s+reference remain unchanged/
    )
    expect(document.operational_contract).to include('Hard deletion is permitted only for a never-used root')
    expect(document.operational_contract).to include(
      'API sync represents retirement without deleting historical identity while'
    )
    expect(document.operational_contract).to match(/preserves the same portable identity\s+\(`portable_id`\)/)
    expect(document.operational_contract).to include(
      'Import and restore preserve retired state and never activate it implicitly.'
    )
  end

  it 'assigns lifecycle ownership and protects the glossary contract link' do
    expect(document.context_map).to include(
      '[Record lifecycle operational contract](../operations/record-lifecycle.md)',
      'Medication Catalogue owns the',
      'Identity owns linked-user account'
    )
    expect(document.glossary).to include(
      '### Retirement',
      '### Logical cold storage',
      '### Reactivation',
      '[record lifecycle contract](operations/record-lifecycle.md)'
    )
  end

  it 'makes the contract discoverable in the documentation index and navigation' do
    expect(document.docs_index).to include('[Record lifecycle contract](operations/record-lifecycle.md)')
    expect(document.navigation).to include('{ "Record lifecycle contract" = "operations/record-lifecycle.md" }')
  end

  it 'includes each required lifecycle acceptance example' do
    contract = document.operational_contract

    expect(contract).to include('A Person with no dependants')
    expect(contract).to include('A Person with dependants')
    expect(contract).to include('A sole carer linked to a user account')
    expect(contract).to include('A never-used root with no protected state')
  end

  it 'protects each root transition invariant' do
    contract = document.operational_contract

    expect(contract).to match(/The selected Medication becomes retired.*no Person changes/m)
    expect(contract).to match(
      /Retiring a Medication retires only that Medication's active `Schedule` and\s+`PersonMedication` rows/
    )
    expect(contract).to match(/Every `MedicationTake`, its source, and its root\s+reference remain unchanged/)
    expect(contract).to include(
      'The selected Location becomes active',
      'The selected Person becomes active',
      'The selected Medication becomes active'
    )
  end

  it 'protects conflict and concurrency invariants' do
    expect(document.operational_contract).to include(
      'HTTP 409 Conflict',
      'location precondition',
      'Repeating a completed same-state transition is idempotent',
      'does not duplicate audit evidence',
      'stale or concurrent state returns the stable conflict envelope',
      'No partial root, child, relationship, or audit writes'
    )
  end

  it 'defines linked-account deactivation authority separately from membership lifecycle' do
    contract = document.operational_contract

    expect(contract).to include(
      'Household authority to manage a Person and linked account',
      'selected HouseholdMembership',
      'Account deactivation is a global Identity transition',
      'separate tenant-scoped audit evidence'
    )
    expect(contract).to include('explicit confirmation token/flag')
  end

  it 'keeps global account deactivation outside single-household authority' do
    contract = document.operational_contract

    expect(contract).to include('never authorizes global Account deactivation')
    expect(contract).to include('selected HouseholdMembership')
    expect(contract).to include('account holder or Identity-level platform operator')
    expect(contract).to include('each affected household')
  end

  it 'protects every acceptance example label in the lifecycle contract' do
    contract = document.operational_contract

    [
      'A medication with active administration sources',
      'A Person with no dependants',
      'A Person with dependants',
      'A sole carer linked to a user account',
      'A Location that is primary or holds active stock',
      'A retired Person, Medication, or Location',
      'A never-used root with no protected state',
      'A repeated or stale retirement request'
    ].each do |example_label|
      expect(contract).to include(example_label)
    end
  end
end
