module Profiles
  class Avatar
    class StorageError < StandardError
    end

    def initialize(person:, authorization:)
      @person = person
      @authorization = authorization
    end

    def upload(file)
      raise Update::InvalidAttributes unless file.is_a?(ActionDispatch::Http::UploadedFile)

      with_current_person_lock do
        replace_image(file)
      end
    end

    def remove
      with_current_person_lock do
        @person.update!(avatar: nil)
        record_change!('removed')
      end
    end

    def download
      @person.avatar.download
    rescue StandardError => e
      storage_failure!(e)
    end

    private

    def with_current_person_lock
      Households::LifecycleCutoffLock.with(household_id: @person.household_id) do
        @person.with_lock(requires_new: true) do
          authorize_current_actor!
          yield
        end
      end
    end

    def authorize_current_actor!
      membership = @authorization.membership.reload
      raise Pundit::NotAuthorizedError unless @person.household.reload.operational?
      unless membership.person_id == @person.id && @person.account_id == @authorization.account.id
        raise Pundit::NotAuthorizedError
      end

      Pundit.authorize(@authorization.with(membership: membership), @person, :update?)
    end

    def replace_image(file)
      blob = ActiveStorage::Blob.build_after_unfurling(io: file.tempfile, filename: file.original_filename,
                                                       content_type: file.content_type)
      @person.avatar = blob
      @person.validate!
      upload_bytes(blob, file.tempfile)
      @person.save!
      record_change!('updated')
    rescue StandardError
      discard_upload(blob) if blob
      raise
    end

    def upload_bytes(blob, io)
      blob.upload_without_unfurling(io)
    rescue StandardError => e
      storage_failure!(e)
    end

    def discard_upload(blob)
      blob.service.delete(blob.key)
    rescue StandardError => e
      Observability::DiagnosticEvent.failure(component: :api_avatar, error: e)
    end

    def storage_failure!(error)
      Observability::DiagnosticEvent.failure(component: :api_avatar, error: error)
      raise StorageError
    end

    def record_change!(action)
      Audit::Event.record!(household: @person.household, event_type: "profile.avatar.#{action}",
                           metadata: { person_id: @person.id })
    end
  end
end
