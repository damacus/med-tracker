require 'rails_helper'

class RecordLifecycleContractDocument
  def operational_contract = Rails.root.join('docs/operations/record-lifecycle.md').read

  def context_map = Rails.root.join('docs/adrs/0009-bounded-context-map.md').read

  def glossary = Rails.root.join('docs/glossary.md').read

  def docs_index = Rails.root.join('docs/index.md').read

  def navigation = Rails.root.join('zensical.toml').read
end

RSpec.describe RecordLifecycleContractDocument do
  subject(:document) { described_class.new }

  it 'defines reversible cold-storage behaviour for medication, person, and location roots' do
    expect(document.operational_contract).to include('Retirement is explicit and reversible.')
    expect(document.operational_contract).to include('Logical cold storage is a lifecycle and visibility state')
    expect(document.operational_contract).to match(/Medication.*Retires only that medication's active.*PersonMedication/m)
    expect(document.operational_contract).to match(/Person.*Retires only that person's own future schedules.*another `Person`/m)
    expect(document.operational_contract).to include('Location retirement is blocked while it is primary or holds active stock')
  end

  it 'protects dependants when a carer is retired' do
    expect(document.operational_contract).to include('lose the last active carer')
    expect(document.operational_contract).to include('requires supervision assignment')
    expect(document.operational_contract).to include('dependant remains active')
    expect(document.operational_contract).to include('needs-carer workflow')
    expect(document.operational_contract).to include('relinking is explicit')
  end

  it 'makes reactivation non-cascading and preserves history and retirement across exchange' do
    expect(document.operational_contract).to include('Reactivation changes only the selected root.')
    expect(document.operational_contract).to match(/never silently restores\s+associated schedules/m)
    expect(document.operational_contract).to include('MedicationTake')
    expect(document.operational_contract).to include('unchanged')
    expect(document.operational_contract).to include('Hard deletion is allowed only for never-used roots')
    expect(document.operational_contract).to match(/Import and restore\s+preserve retired state/m)
  end

  it 'assigns lifecycle ownership and defines the shared vocabulary' do
    expect(document.context_map).to include('[Record lifecycle operational contract](../operations/record-lifecycle.md)')
    expect(document.context_map).to include('Record lifecycle ownership')
    expect(document.glossary).to include('### Retirement')
    expect(document.glossary).to include('### Logical cold storage')
    expect(document.glossary).to include('### Reactivation')
    expect(document.glossary).to include('must not cascade to dependent records')
  end

  it 'makes the contract discoverable in the documentation index and navigation' do
    expect(document.docs_index).to include('[Record lifecycle contract](operations/record-lifecycle.md)')
    expect(document.navigation).to include('{ "Record lifecycle contract" = "operations/record-lifecycle.md" }')
  end
end
