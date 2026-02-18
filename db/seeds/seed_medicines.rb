# frozen_string_literal: true

medicines_file = Rails.root.join('db/seeds/medicines.yml')
medicines_data = YAML.load_file(medicines_file)

medicines_data.each do |attrs|
  medicine = Medicine.find_or_initialize_by(name: attrs['name'])
  medicine.update!(
    dosage_amount: attrs['dosage_amount'],
    dosage_unit: attrs['dosage_unit'],
    current_supply: attrs['current_supply'],
    stock: attrs['stock'],
    reorder_threshold: attrs['reorder_threshold'],
    description: attrs['description'],
    warnings: attrs['warnings']
  )
end

Rails.logger.debug { "Seeded #{medicines_data.size} medicines." }
