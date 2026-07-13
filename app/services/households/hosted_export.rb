# frozen_string_literal: true

module Households
  class HostedExport
    DEFAULT_RETENTION_DAYS = 30

    class << self
      def generate!(household:, membership:, actor_account:)
        authorize_membership!(household, membership, actor_account)
        export, failure = generate_with_failure(household, membership, actor_account)
        raise failure if failure

        export
      end

      def download!(export:, actor_account:)
        authorize_export_actor!(export, actor_account)
        TenantContext.with(account: actor_account, household: export.household) do
          export.with_lock do
            raise ActiveRecord::RecordNotFound unless export.ready? || export.downloaded?
            raise ActiveRecord::RecordNotFound if export.expires_at <= Time.current

            bytes = export.artifact.download
            export.update!(status: :downloaded, downloaded_at: Time.current)
            record_event(export, actor_account, 'downloaded')
            bytes
          end
        end
      end

      def expire!(export:, actor_account:)
        authorize_export_actor!(export, actor_account)
        TenantContext.with(account: actor_account, household: export.household) do
          export.with_lock do
            return export if export.expired?

            expire_export!(export, actor_account)
          end
        end
      end

      def expire_due!(export:)
        export.with_lock do
          return false unless (export.ready? || export.downloaded?) && export.expires_at <= Time.current
          return false if HouseholdRetentionHold.active.exists?(household: export.household)

          expire_export!(export, nil)
          true
        end
      end

      private

      def generate_with_failure(household, membership, actor_account)
        TenantContext.with(account: actor_account, household: household, membership: membership) do
          export = request_export(household, actor_account)
          export.update!(status: :generating, generation_started_at: Time.current)
          record_event(export, actor_account, 'generating')
          generate_or_fail(export, membership, actor_account)
        end
      end

      def generate_or_fail(export, membership, actor_account)
        [generate_archive!(export, membership, actor_account), nil]
      rescue StandardError => e
        fail_export!(export, actor_account, e)
        [export, e]
      end

      def authorize_membership!(household, membership, actor_account)
        manager = membership&.household_id == household.id && membership.active? &&
                  (membership.owner? || membership.administrator?)
        return if manager && actor_authorized?(household, membership, actor_account)

        raise Pundit::NotAuthorizedError, 'Active household manager access is required'
      end

      def actor_authorized?(household, membership, actor_account)
        return true if membership.account_id == actor_account&.id

        platform_admin = actor_account&.platform_admin
        platform_admin&.active? && platform_admin.support_access_sessions.active.exists?(household: household)
      end

      def authorize_export_actor!(export, actor_account)
        authorized = TenantContext.with(account: actor_account, household: export.household) do
          authorized_household_manager?(export.household, actor_account) ||
            authorized_platform_operator?(export.household, actor_account)
        end
        return if authorized

        raise Pundit::NotAuthorizedError, 'Export access is not authorized'
      end

      def authorized_household_manager?(household, actor_account)
        household.operational? && household.household_memberships.active
                                            .where(account: actor_account, role: %i[owner administrator]).exists?
      end

      def authorized_platform_operator?(household, actor_account)
        platform_admin = actor_account&.platform_admin
        platform_admin&.active? && platform_admin.support_access_sessions.active.exists?(household: household)
      end

      def request_export(household, actor_account)
        export = HouseholdExport.create!(
          household: household,
          requested_by_account: actor_account,
          requested_at: Time.current,
          expires_at: retention_days.days.from_now
        )
        record_event(export, actor_account, 'requested')
        export
      end

      def generate_archive!(export, membership, actor_account)
        payload = portable_payload(export, membership)
        attachments, entries = attachment_entries(export.household)
        manifest = export_manifest(export, payload, attachments)
        archive = PortableData::ZipArchive.build(archive_entries(payload, manifest, entries))
        attach_archive(export, archive)
        complete_export!(export, actor_account, manifest, archive)
      end

      def portable_payload(export, membership)
        PortableData::Exporter.new(
          household: export.household,
          membership: membership,
          passphrase: nil
        ).household_payload
      end

      def export_manifest(export, payload, attachments)
        {
          'format' => 'medtracker.household-export.v1',
          'export_id' => export.id,
          'portable_format' => payload.fetch(:format),
          'record_counts' => payload.fetch(:records).transform_values(&:size),
          'attachments' => attachments
        }
      end

      def archive_entries(payload, manifest, attachments)
        {
          'portable.json' => JSON.generate(payload),
          'manifest.json' => JSON.generate(manifest)
        }.merge(attachments)
      end

      def attach_archive(export, archive)
        export.artifact.attach(
          io: StringIO.new(archive),
          filename: "household-export-#{export.id}.zip",
          content_type: 'application/zip'
        )
      end

      def complete_export!(export, actor_account, manifest, archive)
        export.update!(
          status: :ready,
          manifest: manifest,
          artifact_checksum_sha256: Digest::SHA256.hexdigest(archive),
          artifact_byte_size: archive.bytesize,
          ready_at: Time.current
        )
        record_event(export, actor_account, 'ready', attachment_count: manifest.fetch('attachments').size)
        export
      end

      def attachment_entries(household)
        manifest = []
        entries = {}
        ActiveStorage::Attachment.where(household: household).where.not(record_type: 'HouseholdExport')
                                 .includes(:blob).order(:id).find_each do |attachment|
          bytes = verified_blob_bytes(attachment.blob)
          path = "attachments/#{attachment.id}.bin"
          manifest << attachment_manifest_entry(attachment, bytes, path)
          entries[path] = bytes
        end
        [manifest, entries]
      end

      def attachment_manifest_entry(attachment, bytes, path)
        {
          'attachment_id' => attachment.id,
          'record_type' => attachment.record_type,
          'record_id' => attachment.record_id,
          'byte_size' => bytes.bytesize,
          'checksum_sha256' => Digest::SHA256.hexdigest(bytes),
          'archive_path' => path
        }
      end

      def verified_blob_bytes(blob)
        bytes = nil
        blob.open { |file| bytes = file.binmode.read }
        raise ActiveStorage::IntegrityError unless bytes.bytesize == blob.byte_size

        bytes
      end

      def fail_export!(export, actor_account, error)
        destroy_artifact!(export)
        export.update!(status: :failed, failed_at: Time.current, failure_code: error.class.name)
        record_event(export, actor_account, 'failed', failure_code: error.class.name)
      end

      def destroy_artifact!(export)
        return unless export.artifact.attached?

        attachment = export.artifact.attachment
        blob = attachment.blob
        attachment.destroy!
        blob.destroy! unless ActiveStorage::Attachment.exists?(blob_id: blob.id)
      end

      def expire_export!(export, actor_account)
        destroy_artifact!(export)
        export.update!(status: :expired, expired_at: Time.current)
        record_event(export, actor_account, 'expired')
        export
      end

      def record_event(export, actor_account, action, metadata = {})
        Audit::Event.record!(
          household: export.household,
          actor_account: actor_account,
          event_type: "household.export.#{action}",
          metadata: { export_id: export.id, outcome: action }.merge(metadata)
        )
      end

      def retention_days
        Integer(ENV.fetch('HOUSEHOLD_EXPORT_RETENTION_DAYS', DEFAULT_RETENTION_DAYS.to_s), 10).clamp(1, 365)
      rescue ArgumentError
        DEFAULT_RETENTION_DAYS
      end
    end
  end
end
