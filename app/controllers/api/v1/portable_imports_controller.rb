# frozen_string_literal: true

module Api
  module V1
    class PortableImportsController < BaseController
      def dry_run
        render json: { data: result_payload(importer(dry_run: true).call) }
      rescue PortableData::Encryptor::Error, PortableData::Importer::Error => e
        render_unprocessable(e.message)
      end

      def create
        result = importer(dry_run: false).call
        render json: { data: result_payload(result) }, status: result.applied? ? :created : :unprocessable_content
      rescue PortableData::Encryptor::Error, PortableData::Importer::Error => e
        render_unprocessable(e.message)
      end

      private

      def importer(dry_run:)
        PortableData::Importer.new(
          household: current_household,
          membership: current_membership,
          envelope: bundle_params,
          passphrase: params.require(:passphrase),
          options: { dry_run: dry_run, request: request }
        )
      end

      def bundle_params
        params.expect(bundle: %i[format encrypted_at cipher kdf salt checksum ciphertext]).to_h
      end

      def result_payload(result)
        {
          applied: result.applied?,
          counts: result.counts,
          conflicts: result.conflicts,
          errors: result.errors
        }
      end
    end
  end
end
