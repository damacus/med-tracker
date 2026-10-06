# frozen_string_literal: true

class AddMedicineLookupSettingsToAppSettings < ActiveRecord::Migration[8.1]
  def change
    change_table :app_settings, bulk: true do |t|
      t.string :medicine_lookup_base_url, null: false, default: 'https://ontology.nhs.uk/production1/fhir'
      t.string :medicine_lookup_token_url, null: false,
                                           default: 'https://ontology.nhs.uk/authorisation/auth/realms/nhs-digital-terminology/protocol/openid-connect/token'
      t.jsonb :medicine_lookup_source_priority, null: false,
                                                default: %w[
                                                  imported_catalog
                                                  local_nhs_dmd
                                                  cached_open_products_facts
                                                  open_products_facts
                                                  curated_catalog
                                                  nhs_dmd
                                                  supplements
                                                ]
    end
  end
end
