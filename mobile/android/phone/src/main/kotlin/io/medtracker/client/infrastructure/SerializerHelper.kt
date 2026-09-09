package io.medtracker.client.infrastructure

import com.squareup.moshi.Moshi
import com.squareup.moshi.adapters.EnumJsonAdapter
import io.medtracker.client.models.AdminWriteForbiddenError
import io.medtracker.client.models.AiMedicationDoseSuggestion
import io.medtracker.client.models.AuthHouseholdMembership
import io.medtracker.client.models.BackupZipResponseData
import io.medtracker.client.models.Capabilities
import io.medtracker.client.models.CapabilityAuthentication
import io.medtracker.client.models.CapabilityCli
import io.medtracker.client.models.CapabilityClientTools
import io.medtracker.client.models.CapabilityFhir
import io.medtracker.client.models.CapabilityMcpServer
import io.medtracker.client.models.CapabilitySync
import io.medtracker.client.models.DataExportResponseData
import io.medtracker.client.models.DosageOption
import io.medtracker.client.models.DosageOptionCreateAttributes
import io.medtracker.client.models.DosageOptionUpdateAttributes
import io.medtracker.client.models.HealthDataExportResponseData
import io.medtracker.client.models.HealthEvent
import io.medtracker.client.models.HealthEventAttributes
import io.medtracker.client.models.HealthEventCreateRequestHealthEvent
import io.medtracker.client.models.HouseholdAdminSettings
import io.medtracker.client.models.HouseholdAdminSettingsAttributes
import io.medtracker.client.models.HouseholdInvitation
import io.medtracker.client.models.HouseholdInvitationAttributes
import io.medtracker.client.models.HouseholdMembership
import io.medtracker.client.models.HouseholdMembershipAttributes
import io.medtracker.client.models.Me
import io.medtracker.client.models.MeAccount
import io.medtracker.client.models.Medication
import io.medtracker.client.models.MedicationCreateAttributes
import io.medtracker.client.models.MedicationLookupBarcodeResolution
import io.medtracker.client.models.MedicationLookupReviewGuidance
import io.medtracker.client.models.MedicationLookupReviewPrompt
import io.medtracker.client.models.MedicationLookupUnavailableError
import io.medtracker.client.models.MedicationTakeCreateRequestMedicationTake
import io.medtracker.client.models.MedicationUpdateAttributes
import io.medtracker.client.models.NativeDeviceTokenAttributes
import io.medtracker.client.models.Person
import io.medtracker.client.models.PersonAccessGrant
import io.medtracker.client.models.PersonAccessGrantAttributes
import io.medtracker.client.models.PersonAttributes
import io.medtracker.client.models.PersonCreateRequestPerson
import io.medtracker.client.models.PersonMedication
import io.medtracker.client.models.PersonMedicationAttributes
import io.medtracker.client.models.PersonMedicationCreateRequestPersonMedication
import io.medtracker.client.models.PersonMedicationReorderRequest
import io.medtracker.client.models.PortableEnvelope
import io.medtracker.client.models.PortableImportConflict
import io.medtracker.client.models.PortableImportConflictField
import io.medtracker.client.models.PortableSnapshot
import io.medtracker.client.models.PushTestFailedError
import io.medtracker.client.models.RateLimitError
import io.medtracker.client.models.Schedule
import io.medtracker.client.models.ScheduleAttributes
import io.medtracker.client.models.ScheduleCreateRequestSchedule
import io.medtracker.client.models.SyncBatchOperation
import io.medtracker.client.models.SyncBatchResult
import io.medtracker.client.models.SyncChange
import io.medtracker.client.models.SyncSnapshot
import io.medtracker.client.models.SyncTombstone

object SerializerHelper {
    fun addEnumUnknownDefaultCase(moshiBuilder: Moshi.Builder): Moshi.Builder {
        return moshiBuilder
<<<<<<< Updated upstream
            .add(io.medtracker.client.models.AcceptedInvitationMembership.Role::class.java, EnumJsonAdapter.create(io.medtracker.client.models.AcceptedInvitationMembership.Role::class.java)
                .withUnknownFallback(io.medtracker.client.models.AcceptedInvitationMembership.Role.unknown_default_open_api))
            .add(io.medtracker.client.models.AdminWriteForbiddenError.Code::class.java, EnumJsonAdapter.create(io.medtracker.client.models.AdminWriteForbiddenError.Code::class.java)
                .withUnknownFallback(io.medtracker.client.models.AdminWriteForbiddenError.Code.unknown_default_open_api))
            .add(io.medtracker.client.models.AiMedicationDoseSuggestion.Unit::class.java, EnumJsonAdapter.create(io.medtracker.client.models.AiMedicationDoseSuggestion.Unit::class.java)
                .withUnknownFallback(io.medtracker.client.models.AiMedicationDoseSuggestion.Unit.unknown_default_open_api))
            .add(io.medtracker.client.models.AiMedicationDoseSuggestion.DefaultDoseCycle::class.java, EnumJsonAdapter.create(io.medtracker.client.models.AiMedicationDoseSuggestion.DefaultDoseCycle::class.java)
                .withUnknownFallback(io.medtracker.client.models.AiMedicationDoseSuggestion.DefaultDoseCycle.unknown_default_open_api))
            .add(io.medtracker.client.models.AuthHouseholdMembership.Role::class.java, EnumJsonAdapter.create(io.medtracker.client.models.AuthHouseholdMembership.Role::class.java)
                .withUnknownFallback(io.medtracker.client.models.AuthHouseholdMembership.Role.unknown_default_open_api))
            .add(io.medtracker.client.models.AuthHouseholdSelectionData.Status::class.java, EnumJsonAdapter.create(io.medtracker.client.models.AuthHouseholdSelectionData.Status::class.java)
                .withUnknownFallback(io.medtracker.client.models.AuthHouseholdSelectionData.Status.unknown_default_open_api))
            .add(io.medtracker.client.models.BackupZipResponseData.ContentType::class.java, EnumJsonAdapter.create(io.medtracker.client.models.BackupZipResponseData.ContentType::class.java)
                .withUnknownFallback(io.medtracker.client.models.BackupZipResponseData.ContentType.unknown_default_open_api))
            .add(io.medtracker.client.models.Capabilities.Format::class.java, EnumJsonAdapter.create(io.medtracker.client.models.Capabilities.Format::class.java)
                .withUnknownFallback(io.medtracker.client.models.Capabilities.Format.unknown_default_open_api))
            .add(io.medtracker.client.models.Capabilities.ApiVersion::class.java, EnumJsonAdapter.create(io.medtracker.client.models.Capabilities.ApiVersion::class.java)
                .withUnknownFallback(io.medtracker.client.models.Capabilities.ApiVersion.unknown_default_open_api))
            .add(io.medtracker.client.models.Capabilities.PortableFormats::class.java, EnumJsonAdapter.create(io.medtracker.client.models.Capabilities.PortableFormats::class.java)
                .withUnknownFallback(io.medtracker.client.models.Capabilities.PortableFormats.unknown_default_open_api))
            .add(io.medtracker.client.models.CapabilityAuthentication.Methods::class.java, EnumJsonAdapter.create(io.medtracker.client.models.CapabilityAuthentication.Methods::class.java)
                .withUnknownFallback(io.medtracker.client.models.CapabilityAuthentication.Methods.unknown_default_open_api))
            .add(io.medtracker.client.models.CapabilityAuthentication.HostedMobile::class.java, EnumJsonAdapter.create(io.medtracker.client.models.CapabilityAuthentication.HostedMobile::class.java)
                .withUnknownFallback(io.medtracker.client.models.CapabilityAuthentication.HostedMobile.unknown_default_open_api))
            .add(io.medtracker.client.models.CapabilityAuthentication.PasswordLogin::class.java, EnumJsonAdapter.create(io.medtracker.client.models.CapabilityAuthentication.PasswordLogin::class.java)
                .withUnknownFallback(io.medtracker.client.models.CapabilityAuthentication.PasswordLogin.unknown_default_open_api))
            .add(io.medtracker.client.models.CapabilityAvatar.Actions::class.java, EnumJsonAdapter.create(io.medtracker.client.models.CapabilityAvatar.Actions::class.java)
                .withUnknownFallback(io.medtracker.client.models.CapabilityAvatar.Actions.unknown_default_open_api))
            .add(io.medtracker.client.models.CapabilityAvatar.ContentTypes::class.java, EnumJsonAdapter.create(io.medtracker.client.models.CapabilityAvatar.ContentTypes::class.java)
                .withUnknownFallback(io.medtracker.client.models.CapabilityAvatar.ContentTypes.unknown_default_open_api))
            .add(io.medtracker.client.models.CapabilityCli.Status::class.java, EnumJsonAdapter.create(io.medtracker.client.models.CapabilityCli.Status::class.java)
                .withUnknownFallback(io.medtracker.client.models.CapabilityCli.Status.unknown_default_open_api))
            .add(io.medtracker.client.models.CapabilityCli.Binary::class.java, EnumJsonAdapter.create(io.medtracker.client.models.CapabilityCli.Binary::class.java)
                .withUnknownFallback(io.medtracker.client.models.CapabilityCli.Binary.unknown_default_open_api))
            .add(io.medtracker.client.models.CapabilityCli.ApiBoundary::class.java, EnumJsonAdapter.create(io.medtracker.client.models.CapabilityCli.ApiBoundary::class.java)
                .withUnknownFallback(io.medtracker.client.models.CapabilityCli.ApiBoundary.unknown_default_open_api))
            .add(io.medtracker.client.models.CapabilityCli.Distribution::class.java, EnumJsonAdapter.create(io.medtracker.client.models.CapabilityCli.Distribution::class.java)
                .withUnknownFallback(io.medtracker.client.models.CapabilityCli.Distribution.unknown_default_open_api))
            .add(io.medtracker.client.models.CapabilityClientTools.Diagnostics::class.java, EnumJsonAdapter.create(io.medtracker.client.models.CapabilityClientTools.Diagnostics::class.java)
                .withUnknownFallback(io.medtracker.client.models.CapabilityClientTools.Diagnostics.unknown_default_open_api))
            .add(io.medtracker.client.models.CapabilityDoseOutcomes.SourceTypes::class.java, EnumJsonAdapter.create(io.medtracker.client.models.CapabilityDoseOutcomes.SourceTypes::class.java)
                .withUnknownFallback(io.medtracker.client.models.CapabilityDoseOutcomes.SourceTypes.unknown_default_open_api))
            .add(io.medtracker.client.models.CapabilityDoseOutcomes.Actions::class.java, EnumJsonAdapter.create(io.medtracker.client.models.CapabilityDoseOutcomes.Actions::class.java)
                .withUnknownFallback(io.medtracker.client.models.CapabilityDoseOutcomes.Actions.unknown_default_open_api))
            .add(io.medtracker.client.models.CapabilityFhir.Version::class.java, EnumJsonAdapter.create(io.medtracker.client.models.CapabilityFhir.Version::class.java)
                .withUnknownFallback(io.medtracker.client.models.CapabilityFhir.Version.unknown_default_open_api))
            .add(io.medtracker.client.models.CapabilityFhir.Resources::class.java, EnumJsonAdapter.create(io.medtracker.client.models.CapabilityFhir.Resources::class.java)
                .withUnknownFallback(io.medtracker.client.models.CapabilityFhir.Resources.unknown_default_open_api))
            .add(io.medtracker.client.models.CapabilityInvitations.Actions::class.java, EnumJsonAdapter.create(io.medtracker.client.models.CapabilityInvitations.Actions::class.java)
                .withUnknownFallback(io.medtracker.client.models.CapabilityInvitations.Actions.unknown_default_open_api))
            .add(io.medtracker.client.models.CapabilityLocationManagement.Actions::class.java, EnumJsonAdapter.create(io.medtracker.client.models.CapabilityLocationManagement.Actions::class.java)
                .withUnknownFallback(io.medtracker.client.models.CapabilityLocationManagement.Actions.unknown_default_open_api))
            .add(io.medtracker.client.models.CapabilityLocationManagement.PersonMemberships::class.java, EnumJsonAdapter.create(io.medtracker.client.models.CapabilityLocationManagement.PersonMemberships::class.java)
                .withUnknownFallback(io.medtracker.client.models.CapabilityLocationManagement.PersonMemberships.unknown_default_open_api))
            .add(io.medtracker.client.models.CapabilityMcpServer.Transport::class.java, EnumJsonAdapter.create(io.medtracker.client.models.CapabilityMcpServer.Transport::class.java)
                .withUnknownFallback(io.medtracker.client.models.CapabilityMcpServer.Transport.unknown_default_open_api))
            .add(io.medtracker.client.models.CapabilityMcpServer.Endpoint::class.java, EnumJsonAdapter.create(io.medtracker.client.models.CapabilityMcpServer.Endpoint::class.java)
                .withUnknownFallback(io.medtracker.client.models.CapabilityMcpServer.Endpoint.unknown_default_open_api))
            .add(io.medtracker.client.models.CapabilityMcpServer.StdioBinary::class.java, EnumJsonAdapter.create(io.medtracker.client.models.CapabilityMcpServer.StdioBinary::class.java)
                .withUnknownFallback(io.medtracker.client.models.CapabilityMcpServer.StdioBinary.unknown_default_open_api))
            .add(io.medtracker.client.models.CapabilityMedicationPausePeriods.Reasons::class.java, EnumJsonAdapter.create(io.medtracker.client.models.CapabilityMedicationPausePeriods.Reasons::class.java)
                .withUnknownFallback(io.medtracker.client.models.CapabilityMedicationPausePeriods.Reasons.unknown_default_open_api))
            .add(io.medtracker.client.models.CapabilityMedicationPausePeriods.EffectiveTime::class.java, EnumJsonAdapter.create(io.medtracker.client.models.CapabilityMedicationPausePeriods.EffectiveTime::class.java)
                .withUnknownFallback(io.medtracker.client.models.CapabilityMedicationPausePeriods.EffectiveTime.unknown_default_open_api))
            .add(io.medtracker.client.models.CapabilityMedicationReviews.Actions::class.java, EnumJsonAdapter.create(io.medtracker.client.models.CapabilityMedicationReviews.Actions::class.java)
                .withUnknownFallback(io.medtracker.client.models.CapabilityMedicationReviews.Actions.unknown_default_open_api))
            .add(io.medtracker.client.models.CapabilityProfile.Actions::class.java, EnumJsonAdapter.create(io.medtracker.client.models.CapabilityProfile.Actions::class.java)
                .withUnknownFallback(io.medtracker.client.models.CapabilityProfile.Actions.unknown_default_open_api))
            .add(io.medtracker.client.models.CapabilityReports.Formats::class.java, EnumJsonAdapter.create(io.medtracker.client.models.CapabilityReports.Formats::class.java)
                .withUnknownFallback(io.medtracker.client.models.CapabilityReports.Formats.unknown_default_open_api))
            .add(io.medtracker.client.models.CapabilityStockRemovals.Actions::class.java, EnumJsonAdapter.create(io.medtracker.client.models.CapabilityStockRemovals.Actions::class.java)
                .withUnknownFallback(io.medtracker.client.models.CapabilityStockRemovals.Actions.unknown_default_open_api))
            .add(io.medtracker.client.models.CapabilitySync.OnlineOnlyResources::class.java, EnumJsonAdapter.create(io.medtracker.client.models.CapabilitySync.OnlineOnlyResources::class.java)
                .withUnknownFallback(io.medtracker.client.models.CapabilitySync.OnlineOnlyResources.unknown_default_open_api))
            .add(io.medtracker.client.models.CapabilitySync.NumericIds::class.java, EnumJsonAdapter.create(io.medtracker.client.models.CapabilitySync.NumericIds::class.java)
                .withUnknownFallback(io.medtracker.client.models.CapabilitySync.NumericIds.unknown_default_open_api))
            .add(io.medtracker.client.models.DataExportResponseData.Format::class.java, EnumJsonAdapter.create(io.medtracker.client.models.DataExportResponseData.Format::class.java)
                .withUnknownFallback(io.medtracker.client.models.DataExportResponseData.Format.unknown_default_open_api))
            .add(io.medtracker.client.models.DataExportResponseData.Scope::class.java, EnumJsonAdapter.create(io.medtracker.client.models.DataExportResponseData.Scope::class.java)
                .withUnknownFallback(io.medtracker.client.models.DataExportResponseData.Scope.unknown_default_open_api))
            .add(io.medtracker.client.models.DataExportResponseData.Cipher::class.java, EnumJsonAdapter.create(io.medtracker.client.models.DataExportResponseData.Cipher::class.java)
                .withUnknownFallback(io.medtracker.client.models.DataExportResponseData.Cipher.unknown_default_open_api))
            .add(io.medtracker.client.models.DataExportResponseData.Kdf::class.java, EnumJsonAdapter.create(io.medtracker.client.models.DataExportResponseData.Kdf::class.java)
                .withUnknownFallback(io.medtracker.client.models.DataExportResponseData.Kdf.unknown_default_open_api))
            .add(io.medtracker.client.models.DataExportResponseData.ContentType::class.java, EnumJsonAdapter.create(io.medtracker.client.models.DataExportResponseData.ContentType::class.java)
                .withUnknownFallback(io.medtracker.client.models.DataExportResponseData.ContentType.unknown_default_open_api))
            .add(io.medtracker.client.models.DosageOption.DefaultDoseCycle::class.java, EnumJsonAdapter.create(io.medtracker.client.models.DosageOption.DefaultDoseCycle::class.java)
                .withUnknownFallback(io.medtracker.client.models.DosageOption.DefaultDoseCycle.unknown_default_open_api))
            .add(io.medtracker.client.models.DosageOptionCreateAttributes.DefaultDoseCycle::class.java, EnumJsonAdapter.create(io.medtracker.client.models.DosageOptionCreateAttributes.DefaultDoseCycle::class.java)
                .withUnknownFallback(io.medtracker.client.models.DosageOptionCreateAttributes.DefaultDoseCycle.unknown_default_open_api))
            .add(io.medtracker.client.models.DosageOptionUpdateAttributes.DefaultDoseCycle::class.java, EnumJsonAdapter.create(io.medtracker.client.models.DosageOptionUpdateAttributes.DefaultDoseCycle::class.java)
                .withUnknownFallback(io.medtracker.client.models.DosageOptionUpdateAttributes.DefaultDoseCycle.unknown_default_open_api))
            .add(io.medtracker.client.models.DoseNotTakenRequestDoseOccurrence.Reason::class.java, EnumJsonAdapter.create(io.medtracker.client.models.DoseNotTakenRequestDoseOccurrence.Reason::class.java)
                .withUnknownFallback(io.medtracker.client.models.DoseNotTakenRequestDoseOccurrence.Reason.unknown_default_open_api))
            .add(io.medtracker.client.models.DoseOccurrence.SourceType::class.java, EnumJsonAdapter.create(io.medtracker.client.models.DoseOccurrence.SourceType::class.java)
                .withUnknownFallback(io.medtracker.client.models.DoseOccurrence.SourceType.unknown_default_open_api))
            .add(io.medtracker.client.models.DoseOccurrence.Outcome::class.java, EnumJsonAdapter.create(io.medtracker.client.models.DoseOccurrence.Outcome::class.java)
                .withUnknownFallback(io.medtracker.client.models.DoseOccurrence.Outcome.unknown_default_open_api))
            .add(io.medtracker.client.models.DoseOccurrence.Reason::class.java, EnumJsonAdapter.create(io.medtracker.client.models.DoseOccurrence.Reason::class.java)
                .withUnknownFallback(io.medtracker.client.models.DoseOccurrence.Reason.unknown_default_open_api))
            .add(io.medtracker.client.models.HealthDataExportResponseData.Format::class.java, EnumJsonAdapter.create(io.medtracker.client.models.HealthDataExportResponseData.Format::class.java)
                .withUnknownFallback(io.medtracker.client.models.HealthDataExportResponseData.Format.unknown_default_open_api))
            .add(io.medtracker.client.models.HealthDataExportResponseData.Scope::class.java, EnumJsonAdapter.create(io.medtracker.client.models.HealthDataExportResponseData.Scope::class.java)
                .withUnknownFallback(io.medtracker.client.models.HealthDataExportResponseData.Scope.unknown_default_open_api))
            .add(io.medtracker.client.models.HealthEvent.EventKind::class.java, EnumJsonAdapter.create(io.medtracker.client.models.HealthEvent.EventKind::class.java)
                .withUnknownFallback(io.medtracker.client.models.HealthEvent.EventKind.unknown_default_open_api))
            .add(io.medtracker.client.models.HealthEvent.Severity::class.java, EnumJsonAdapter.create(io.medtracker.client.models.HealthEvent.Severity::class.java)
                .withUnknownFallback(io.medtracker.client.models.HealthEvent.Severity.unknown_default_open_api))
            .add(io.medtracker.client.models.HealthEventAttributes.EventKind::class.java, EnumJsonAdapter.create(io.medtracker.client.models.HealthEventAttributes.EventKind::class.java)
                .withUnknownFallback(io.medtracker.client.models.HealthEventAttributes.EventKind.unknown_default_open_api))
            .add(io.medtracker.client.models.HealthEventAttributes.Severity::class.java, EnumJsonAdapter.create(io.medtracker.client.models.HealthEventAttributes.Severity::class.java)
                .withUnknownFallback(io.medtracker.client.models.HealthEventAttributes.Severity.unknown_default_open_api))
            .add(io.medtracker.client.models.HealthEventCreateRequestHealthEvent.EventKind::class.java, EnumJsonAdapter.create(io.medtracker.client.models.HealthEventCreateRequestHealthEvent.EventKind::class.java)
                .withUnknownFallback(io.medtracker.client.models.HealthEventCreateRequestHealthEvent.EventKind.unknown_default_open_api))
            .add(io.medtracker.client.models.HealthEventCreateRequestHealthEvent.Severity::class.java, EnumJsonAdapter.create(io.medtracker.client.models.HealthEventCreateRequestHealthEvent.Severity::class.java)
                .withUnknownFallback(io.medtracker.client.models.HealthEventCreateRequestHealthEvent.Severity.unknown_default_open_api))
            .add(io.medtracker.client.models.HouseholdAdminSettings.SubscriptionPlan::class.java, EnumJsonAdapter.create(io.medtracker.client.models.HouseholdAdminSettings.SubscriptionPlan::class.java)
                .withUnknownFallback(io.medtracker.client.models.HouseholdAdminSettings.SubscriptionPlan.unknown_default_open_api))
            .add(io.medtracker.client.models.HouseholdAdminSettingsAttributes.SubscriptionPlan::class.java, EnumJsonAdapter.create(io.medtracker.client.models.HouseholdAdminSettingsAttributes.SubscriptionPlan::class.java)
                .withUnknownFallback(io.medtracker.client.models.HouseholdAdminSettingsAttributes.SubscriptionPlan.unknown_default_open_api))
            .add(io.medtracker.client.models.HouseholdInvitation.MembershipRole::class.java, EnumJsonAdapter.create(io.medtracker.client.models.HouseholdInvitation.MembershipRole::class.java)
                .withUnknownFallback(io.medtracker.client.models.HouseholdInvitation.MembershipRole.unknown_default_open_api))
            .add(io.medtracker.client.models.HouseholdInvitationAttributes.MembershipRole::class.java, EnumJsonAdapter.create(io.medtracker.client.models.HouseholdInvitationAttributes.MembershipRole::class.java)
                .withUnknownFallback(io.medtracker.client.models.HouseholdInvitationAttributes.MembershipRole.unknown_default_open_api))
            .add(io.medtracker.client.models.HouseholdMembership.Role::class.java, EnumJsonAdapter.create(io.medtracker.client.models.HouseholdMembership.Role::class.java)
                .withUnknownFallback(io.medtracker.client.models.HouseholdMembership.Role.unknown_default_open_api))
            .add(io.medtracker.client.models.HouseholdMembership.Status::class.java, EnumJsonAdapter.create(io.medtracker.client.models.HouseholdMembership.Status::class.java)
                .withUnknownFallback(io.medtracker.client.models.HouseholdMembership.Status.unknown_default_open_api))
            .add(io.medtracker.client.models.HouseholdMembershipAttributes.Role::class.java, EnumJsonAdapter.create(io.medtracker.client.models.HouseholdMembershipAttributes.Role::class.java)
                .withUnknownFallback(io.medtracker.client.models.HouseholdMembershipAttributes.Role.unknown_default_open_api))
            .add(io.medtracker.client.models.HouseholdMembershipAttributes.Status::class.java, EnumJsonAdapter.create(io.medtracker.client.models.HouseholdMembershipAttributes.Status::class.java)
                .withUnknownFallback(io.medtracker.client.models.HouseholdMembershipAttributes.Status.unknown_default_open_api))
            .add(io.medtracker.client.models.HouseholdProfile.MobileShortcuts::class.java, EnumJsonAdapter.create(io.medtracker.client.models.HouseholdProfile.MobileShortcuts::class.java)
                .withUnknownFallback(io.medtracker.client.models.HouseholdProfile.MobileShortcuts.unknown_default_open_api))
            .add(io.medtracker.client.models.HouseholdProfileUpdate.MobileShortcuts::class.java, EnumJsonAdapter.create(io.medtracker.client.models.HouseholdProfileUpdate.MobileShortcuts::class.java)
                .withUnknownFallback(io.medtracker.client.models.HouseholdProfileUpdate.MobileShortcuts.unknown_default_open_api))
            .add(io.medtracker.client.models.InvitationResendResult.DeliveryStatus::class.java, EnumJsonAdapter.create(io.medtracker.client.models.InvitationResendResult.DeliveryStatus::class.java)
                .withUnknownFallback(io.medtracker.client.models.InvitationResendResult.DeliveryStatus.unknown_default_open_api))
            .add(io.medtracker.client.models.Me.MembershipRole::class.java, EnumJsonAdapter.create(io.medtracker.client.models.Me.MembershipRole::class.java)
                .withUnknownFallback(io.medtracker.client.models.Me.MembershipRole.unknown_default_open_api))
            .add(io.medtracker.client.models.MeAccount.Status::class.java, EnumJsonAdapter.create(io.medtracker.client.models.MeAccount.Status::class.java)
                .withUnknownFallback(io.medtracker.client.models.MeAccount.Status.unknown_default_open_api))
            .add(io.medtracker.client.models.Medication.ReorderStatus::class.java, EnumJsonAdapter.create(io.medtracker.client.models.Medication.ReorderStatus::class.java)
                .withUnknownFallback(io.medtracker.client.models.Medication.ReorderStatus.unknown_default_open_api))
            .add(io.medtracker.client.models.MedicationCreateAttributes.DoseUnit::class.java, EnumJsonAdapter.create(io.medtracker.client.models.MedicationCreateAttributes.DoseUnit::class.java)
                .withUnknownFallback(io.medtracker.client.models.MedicationCreateAttributes.DoseUnit.unknown_default_open_api))
            .add(io.medtracker.client.models.MedicationCreateAttributes.DefaultScheduleType::class.java, EnumJsonAdapter.create(io.medtracker.client.models.MedicationCreateAttributes.DefaultScheduleType::class.java)
                .withUnknownFallback(io.medtracker.client.models.MedicationCreateAttributes.DefaultScheduleType.unknown_default_open_api))
            .add(io.medtracker.client.models.MedicationLookupBarcodeResolution.Status::class.java, EnumJsonAdapter.create(io.medtracker.client.models.MedicationLookupBarcodeResolution.Status::class.java)
                .withUnknownFallback(io.medtracker.client.models.MedicationLookupBarcodeResolution.Status.unknown_default_open_api))
            .add(io.medtracker.client.models.MedicationLookupReviewGuidance.Status::class.java, EnumJsonAdapter.create(io.medtracker.client.models.MedicationLookupReviewGuidance.Status::class.java)
                .withUnknownFallback(io.medtracker.client.models.MedicationLookupReviewGuidance.Status.unknown_default_open_api))
            .add(io.medtracker.client.models.MedicationLookupReviewPrompt.RiskLevel::class.java, EnumJsonAdapter.create(io.medtracker.client.models.MedicationLookupReviewPrompt.RiskLevel::class.java)
                .withUnknownFallback(io.medtracker.client.models.MedicationLookupReviewPrompt.RiskLevel.unknown_default_open_api))
            .add(io.medtracker.client.models.MedicationLookupReviewPrompt.MatchConfidence::class.java, EnumJsonAdapter.create(io.medtracker.client.models.MedicationLookupReviewPrompt.MatchConfidence::class.java)
                .withUnknownFallback(io.medtracker.client.models.MedicationLookupReviewPrompt.MatchConfidence.unknown_default_open_api))
            .add(io.medtracker.client.models.MedicationLookupReviewPrompt.MatchType::class.java, EnumJsonAdapter.create(io.medtracker.client.models.MedicationLookupReviewPrompt.MatchType::class.java)
                .withUnknownFallback(io.medtracker.client.models.MedicationLookupReviewPrompt.MatchType.unknown_default_open_api))
            .add(io.medtracker.client.models.MedicationLookupUnavailableError::class.java, EnumJsonAdapter.create(io.medtracker.client.models.MedicationLookupUnavailableError::class.java)
                .withUnknownFallback(io.medtracker.client.models.MedicationLookupUnavailableError.unknown_default_open_api))
            .add(io.medtracker.client.models.MedicationPausePeriod.SourceType::class.java, EnumJsonAdapter.create(io.medtracker.client.models.MedicationPausePeriod.SourceType::class.java)
                .withUnknownFallback(io.medtracker.client.models.MedicationPausePeriod.SourceType.unknown_default_open_api))
            .add(io.medtracker.client.models.MedicationPausePeriod.Reason::class.java, EnumJsonAdapter.create(io.medtracker.client.models.MedicationPausePeriod.Reason::class.java)
                .withUnknownFallback(io.medtracker.client.models.MedicationPausePeriod.Reason.unknown_default_open_api))
            .add(io.medtracker.client.models.MedicationPausePeriodCreateRequestMedicationPausePeriod.SourceType::class.java, EnumJsonAdapter.create(io.medtracker.client.models.MedicationPausePeriodCreateRequestMedicationPausePeriod.SourceType::class.java)
                .withUnknownFallback(io.medtracker.client.models.MedicationPausePeriodCreateRequestMedicationPausePeriod.SourceType.unknown_default_open_api))
            .add(io.medtracker.client.models.MedicationPausePeriodCreateRequestMedicationPausePeriod.Reason::class.java, EnumJsonAdapter.create(io.medtracker.client.models.MedicationPausePeriodCreateRequestMedicationPausePeriod.Reason::class.java)
                .withUnknownFallback(io.medtracker.client.models.MedicationPausePeriodCreateRequestMedicationPausePeriod.Reason.unknown_default_open_api))
            .add(io.medtracker.client.models.MedicationReviewPrompt.RiskLevel::class.java, EnumJsonAdapter.create(io.medtracker.client.models.MedicationReviewPrompt.RiskLevel::class.java)
                .withUnknownFallback(io.medtracker.client.models.MedicationReviewPrompt.RiskLevel.unknown_default_open_api))
            .add(io.medtracker.client.models.MedicationReviewPrompt.MatchConfidence::class.java, EnumJsonAdapter.create(io.medtracker.client.models.MedicationReviewPrompt.MatchConfidence::class.java)
                .withUnknownFallback(io.medtracker.client.models.MedicationReviewPrompt.MatchConfidence.unknown_default_open_api))
            .add(io.medtracker.client.models.MedicationReviewPromptStatus::class.java, EnumJsonAdapter.create(io.medtracker.client.models.MedicationReviewPromptStatus::class.java)
                .withUnknownFallback(io.medtracker.client.models.MedicationReviewPromptStatus.unknown_default_open_api))
            .add(io.medtracker.client.models.MedicationTakeCreateRequestMedicationTake.SourceType::class.java, EnumJsonAdapter.create(io.medtracker.client.models.MedicationTakeCreateRequestMedicationTake.SourceType::class.java)
                .withUnknownFallback(io.medtracker.client.models.MedicationTakeCreateRequestMedicationTake.SourceType.unknown_default_open_api))
            .add(io.medtracker.client.models.MedicationUpdateAttributes.DoseUnit::class.java, EnumJsonAdapter.create(io.medtracker.client.models.MedicationUpdateAttributes.DoseUnit::class.java)
                .withUnknownFallback(io.medtracker.client.models.MedicationUpdateAttributes.DoseUnit.unknown_default_open_api))
            .add(io.medtracker.client.models.MedicationUpdateAttributes.DefaultScheduleType::class.java, EnumJsonAdapter.create(io.medtracker.client.models.MedicationUpdateAttributes.DefaultScheduleType::class.java)
                .withUnknownFallback(io.medtracker.client.models.MedicationUpdateAttributes.DefaultScheduleType.unknown_default_open_api))
            .add(io.medtracker.client.models.NativeDeviceTokenAttributes.Platform::class.java, EnumJsonAdapter.create(io.medtracker.client.models.NativeDeviceTokenAttributes.Platform::class.java)
                .withUnknownFallback(io.medtracker.client.models.NativeDeviceTokenAttributes.Platform.unknown_default_open_api))
            .add(io.medtracker.client.models.NativeDeviceTokenAttributes.ApnsEnvironment::class.java, EnumJsonAdapter.create(io.medtracker.client.models.NativeDeviceTokenAttributes.ApnsEnvironment::class.java)
                .withUnknownFallback(io.medtracker.client.models.NativeDeviceTokenAttributes.ApnsEnvironment.unknown_default_open_api))
            .add(io.medtracker.client.models.NullableMedicationPausePeriod.SourceType::class.java, EnumJsonAdapter.create(io.medtracker.client.models.NullableMedicationPausePeriod.SourceType::class.java)
                .withUnknownFallback(io.medtracker.client.models.NullableMedicationPausePeriod.SourceType.unknown_default_open_api))
            .add(io.medtracker.client.models.NullableMedicationPausePeriod.Reason::class.java, EnumJsonAdapter.create(io.medtracker.client.models.NullableMedicationPausePeriod.Reason::class.java)
                .withUnknownFallback(io.medtracker.client.models.NullableMedicationPausePeriod.Reason.unknown_default_open_api))
            .add(io.medtracker.client.models.Person.PersonType::class.java, EnumJsonAdapter.create(io.medtracker.client.models.Person.PersonType::class.java)
                .withUnknownFallback(io.medtracker.client.models.Person.PersonType.unknown_default_open_api))
            .add(io.medtracker.client.models.PersonAccessGrant.AccessLevel::class.java, EnumJsonAdapter.create(io.medtracker.client.models.PersonAccessGrant.AccessLevel::class.java)
                .withUnknownFallback(io.medtracker.client.models.PersonAccessGrant.AccessLevel.unknown_default_open_api))
            .add(io.medtracker.client.models.PersonAccessGrant.RelationshipType::class.java, EnumJsonAdapter.create(io.medtracker.client.models.PersonAccessGrant.RelationshipType::class.java)
                .withUnknownFallback(io.medtracker.client.models.PersonAccessGrant.RelationshipType.unknown_default_open_api))
            .add(io.medtracker.client.models.PersonAccessGrantAttributes.AccessLevel::class.java, EnumJsonAdapter.create(io.medtracker.client.models.PersonAccessGrantAttributes.AccessLevel::class.java)
                .withUnknownFallback(io.medtracker.client.models.PersonAccessGrantAttributes.AccessLevel.unknown_default_open_api))
            .add(io.medtracker.client.models.PersonAccessGrantAttributes.RelationshipType::class.java, EnumJsonAdapter.create(io.medtracker.client.models.PersonAccessGrantAttributes.RelationshipType::class.java)
                .withUnknownFallback(io.medtracker.client.models.PersonAccessGrantAttributes.RelationshipType.unknown_default_open_api))
            .add(io.medtracker.client.models.PersonAttributes.PersonType::class.java, EnumJsonAdapter.create(io.medtracker.client.models.PersonAttributes.PersonType::class.java)
                .withUnknownFallback(io.medtracker.client.models.PersonAttributes.PersonType.unknown_default_open_api))
            .add(io.medtracker.client.models.PersonCreateRequestPerson.PersonType::class.java, EnumJsonAdapter.create(io.medtracker.client.models.PersonCreateRequestPerson.PersonType::class.java)
                .withUnknownFallback(io.medtracker.client.models.PersonCreateRequestPerson.PersonType.unknown_default_open_api))
            .add(io.medtracker.client.models.PersonMedication.DoseCycle::class.java, EnumJsonAdapter.create(io.medtracker.client.models.PersonMedication.DoseCycle::class.java)
                .withUnknownFallback(io.medtracker.client.models.PersonMedication.DoseCycle.unknown_default_open_api))
            .add(io.medtracker.client.models.PersonMedication.AdministrationKind::class.java, EnumJsonAdapter.create(io.medtracker.client.models.PersonMedication.AdministrationKind::class.java)
                .withUnknownFallback(io.medtracker.client.models.PersonMedication.AdministrationKind.unknown_default_open_api))
            .add(io.medtracker.client.models.PersonMedicationAttributes.AdministrationKind::class.java, EnumJsonAdapter.create(io.medtracker.client.models.PersonMedicationAttributes.AdministrationKind::class.java)
                .withUnknownFallback(io.medtracker.client.models.PersonMedicationAttributes.AdministrationKind.unknown_default_open_api))
            .add(io.medtracker.client.models.PersonMedicationAttributes.DoseCycle::class.java, EnumJsonAdapter.create(io.medtracker.client.models.PersonMedicationAttributes.DoseCycle::class.java)
                .withUnknownFallback(io.medtracker.client.models.PersonMedicationAttributes.DoseCycle.unknown_default_open_api))
            .add(io.medtracker.client.models.PersonMedicationCreateRequestPersonMedication.AdministrationKind::class.java, EnumJsonAdapter.create(io.medtracker.client.models.PersonMedicationCreateRequestPersonMedication.AdministrationKind::class.java)
                .withUnknownFallback(io.medtracker.client.models.PersonMedicationCreateRequestPersonMedication.AdministrationKind.unknown_default_open_api))
            .add(io.medtracker.client.models.PersonMedicationCreateRequestPersonMedication.DoseCycle::class.java, EnumJsonAdapter.create(io.medtracker.client.models.PersonMedicationCreateRequestPersonMedication.DoseCycle::class.java)
                .withUnknownFallback(io.medtracker.client.models.PersonMedicationCreateRequestPersonMedication.DoseCycle.unknown_default_open_api))
            .add(io.medtracker.client.models.PersonMedicationReorderRequest.Direction::class.java, EnumJsonAdapter.create(io.medtracker.client.models.PersonMedicationReorderRequest.Direction::class.java)
                .withUnknownFallback(io.medtracker.client.models.PersonMedicationReorderRequest.Direction.unknown_default_open_api))
            .add(io.medtracker.client.models.PortableDoseOccurrence.SourceType::class.java, EnumJsonAdapter.create(io.medtracker.client.models.PortableDoseOccurrence.SourceType::class.java)
                .withUnknownFallback(io.medtracker.client.models.PortableDoseOccurrence.SourceType.unknown_default_open_api))
            .add(io.medtracker.client.models.PortableDoseOccurrence.Outcome::class.java, EnumJsonAdapter.create(io.medtracker.client.models.PortableDoseOccurrence.Outcome::class.java)
                .withUnknownFallback(io.medtracker.client.models.PortableDoseOccurrence.Outcome.unknown_default_open_api))
            .add(io.medtracker.client.models.PortableDoseOccurrence.Reason::class.java, EnumJsonAdapter.create(io.medtracker.client.models.PortableDoseOccurrence.Reason::class.java)
                .withUnknownFallback(io.medtracker.client.models.PortableDoseOccurrence.Reason.unknown_default_open_api))
            .add(io.medtracker.client.models.PortableEnvelope.Format::class.java, EnumJsonAdapter.create(io.medtracker.client.models.PortableEnvelope.Format::class.java)
                .withUnknownFallback(io.medtracker.client.models.PortableEnvelope.Format.unknown_default_open_api))
            .add(io.medtracker.client.models.PortableEnvelope.Cipher::class.java, EnumJsonAdapter.create(io.medtracker.client.models.PortableEnvelope.Cipher::class.java)
                .withUnknownFallback(io.medtracker.client.models.PortableEnvelope.Cipher.unknown_default_open_api))
            .add(io.medtracker.client.models.PortableEnvelope.Kdf::class.java, EnumJsonAdapter.create(io.medtracker.client.models.PortableEnvelope.Kdf::class.java)
                .withUnknownFallback(io.medtracker.client.models.PortableEnvelope.Kdf.unknown_default_open_api))
            .add(io.medtracker.client.models.PortableImportConflict.RecordType::class.java, EnumJsonAdapter.create(io.medtracker.client.models.PortableImportConflict.RecordType::class.java)
                .withUnknownFallback(io.medtracker.client.models.PortableImportConflict.RecordType.unknown_default_open_api))
            .add(io.medtracker.client.models.PortableImportConflictField::class.java, EnumJsonAdapter.create(io.medtracker.client.models.PortableImportConflictField::class.java)
                .withUnknownFallback(io.medtracker.client.models.PortableImportConflictField.unknown_default_open_api))
            .add(io.medtracker.client.models.PortableSnapshot.Format::class.java, EnumJsonAdapter.create(io.medtracker.client.models.PortableSnapshot.Format::class.java)
                .withUnknownFallback(io.medtracker.client.models.PortableSnapshot.Format.unknown_default_open_api))
            .add(io.medtracker.client.models.PortableSnapshot.Scope::class.java, EnumJsonAdapter.create(io.medtracker.client.models.PortableSnapshot.Scope::class.java)
                .withUnknownFallback(io.medtracker.client.models.PortableSnapshot.Scope.unknown_default_open_api))
            .add(io.medtracker.client.models.PushTestFailedError.Code::class.java, EnumJsonAdapter.create(io.medtracker.client.models.PushTestFailedError.Code::class.java)
                .withUnknownFallback(io.medtracker.client.models.PushTestFailedError.Code.unknown_default_open_api))
            .add(io.medtracker.client.models.PushTestFailedError.Message::class.java, EnumJsonAdapter.create(io.medtracker.client.models.PushTestFailedError.Message::class.java)
                .withUnknownFallback(io.medtracker.client.models.PushTestFailedError.Message.unknown_default_open_api))
            .add(io.medtracker.client.models.RateLimitError.Code::class.java, EnumJsonAdapter.create(io.medtracker.client.models.RateLimitError.Code::class.java)
                .withUnknownFallback(io.medtracker.client.models.RateLimitError.Code.unknown_default_open_api))
            .add(io.medtracker.client.models.ReportHealthEvent.EventKind::class.java, EnumJsonAdapter.create(io.medtracker.client.models.ReportHealthEvent.EventKind::class.java)
                .withUnknownFallback(io.medtracker.client.models.ReportHealthEvent.EventKind.unknown_default_open_api))
            .add(io.medtracker.client.models.ReportHealthEvent.Severity::class.java, EnumJsonAdapter.create(io.medtracker.client.models.ReportHealthEvent.Severity::class.java)
                .withUnknownFallback(io.medtracker.client.models.ReportHealthEvent.Severity.unknown_default_open_api))
            .add(io.medtracker.client.models.ReportMedicationTake.SourceType::class.java, EnumJsonAdapter.create(io.medtracker.client.models.ReportMedicationTake.SourceType::class.java)
                .withUnknownFallback(io.medtracker.client.models.ReportMedicationTake.SourceType.unknown_default_open_api))
            .add(io.medtracker.client.models.Schedule.DoseCycle::class.java, EnumJsonAdapter.create(io.medtracker.client.models.Schedule.DoseCycle::class.java)
                .withUnknownFallback(io.medtracker.client.models.Schedule.DoseCycle.unknown_default_open_api))
            .add(io.medtracker.client.models.Schedule.ScheduleType::class.java, EnumJsonAdapter.create(io.medtracker.client.models.Schedule.ScheduleType::class.java)
                .withUnknownFallback(io.medtracker.client.models.Schedule.ScheduleType.unknown_default_open_api))
            .add(io.medtracker.client.models.ScheduleAttributes.DoseCycle::class.java, EnumJsonAdapter.create(io.medtracker.client.models.ScheduleAttributes.DoseCycle::class.java)
                .withUnknownFallback(io.medtracker.client.models.ScheduleAttributes.DoseCycle.unknown_default_open_api))
            .add(io.medtracker.client.models.ScheduleAttributes.ScheduleType::class.java, EnumJsonAdapter.create(io.medtracker.client.models.ScheduleAttributes.ScheduleType::class.java)
                .withUnknownFallback(io.medtracker.client.models.ScheduleAttributes.ScheduleType.unknown_default_open_api))
            .add(io.medtracker.client.models.ScheduleCreateRequestSchedule.DoseCycle::class.java, EnumJsonAdapter.create(io.medtracker.client.models.ScheduleCreateRequestSchedule.DoseCycle::class.java)
                .withUnknownFallback(io.medtracker.client.models.ScheduleCreateRequestSchedule.DoseCycle.unknown_default_open_api))
            .add(io.medtracker.client.models.ScheduleCreateRequestSchedule.ScheduleType::class.java, EnumJsonAdapter.create(io.medtracker.client.models.ScheduleCreateRequestSchedule.ScheduleType::class.java)
                .withUnknownFallback(io.medtracker.client.models.ScheduleCreateRequestSchedule.ScheduleType.unknown_default_open_api))
            .add(io.medtracker.client.models.StockRemovalReason::class.java, EnumJsonAdapter.create(io.medtracker.client.models.StockRemovalReason::class.java)
                .withUnknownFallback(io.medtracker.client.models.StockRemovalReason.unknown_default_open_api))
            .add(io.medtracker.client.models.SyncBatchOperation.Action::class.java, EnumJsonAdapter.create(io.medtracker.client.models.SyncBatchOperation.Action::class.java)
                .withUnknownFallback(io.medtracker.client.models.SyncBatchOperation.Action.unknown_default_open_api))
            .add(io.medtracker.client.models.SyncBatchOperation.ResourceType::class.java, EnumJsonAdapter.create(io.medtracker.client.models.SyncBatchOperation.ResourceType::class.java)
                .withUnknownFallback(io.medtracker.client.models.SyncBatchOperation.ResourceType.unknown_default_open_api))
            .add(io.medtracker.client.models.SyncBatchResult.Action::class.java, EnumJsonAdapter.create(io.medtracker.client.models.SyncBatchResult.Action::class.java)
                .withUnknownFallback(io.medtracker.client.models.SyncBatchResult.Action.unknown_default_open_api))
            .add(io.medtracker.client.models.SyncBatchResult.RecordType::class.java, EnumJsonAdapter.create(io.medtracker.client.models.SyncBatchResult.RecordType::class.java)
                .withUnknownFallback(io.medtracker.client.models.SyncBatchResult.RecordType.unknown_default_open_api))
            .add(io.medtracker.client.models.SyncChange.Action::class.java, EnumJsonAdapter.create(io.medtracker.client.models.SyncChange.Action::class.java)
                .withUnknownFallback(io.medtracker.client.models.SyncChange.Action.unknown_default_open_api))
            .add(io.medtracker.client.models.SyncOperationCapability.ResourceType::class.java, EnumJsonAdapter.create(io.medtracker.client.models.SyncOperationCapability.ResourceType::class.java)
                .withUnknownFallback(io.medtracker.client.models.SyncOperationCapability.ResourceType.unknown_default_open_api))
            .add(io.medtracker.client.models.SyncOperationCapability.Actions::class.java, EnumJsonAdapter.create(io.medtracker.client.models.SyncOperationCapability.Actions::class.java)
                .withUnknownFallback(io.medtracker.client.models.SyncOperationCapability.Actions.unknown_default_open_api))
            .add(io.medtracker.client.models.SyncSnapshot.Format::class.java, EnumJsonAdapter.create(io.medtracker.client.models.SyncSnapshot.Format::class.java)
                .withUnknownFallback(io.medtracker.client.models.SyncSnapshot.Format.unknown_default_open_api))
            .add(io.medtracker.client.models.SyncSnapshot.Scope::class.java, EnumJsonAdapter.create(io.medtracker.client.models.SyncSnapshot.Scope::class.java)
                .withUnknownFallback(io.medtracker.client.models.SyncSnapshot.Scope.unknown_default_open_api))
            .add(io.medtracker.client.models.SyncTombstone.Action::class.java, EnumJsonAdapter.create(io.medtracker.client.models.SyncTombstone.Action::class.java)
                .withUnknownFallback(io.medtracker.client.models.SyncTombstone.Action.unknown_default_open_api))
=======
            .add(
	            AdminWriteForbiddenError.Code::class.java, EnumJsonAdapter.create(
		            AdminWriteForbiddenError.Code::class.java)
                .withUnknownFallback(AdminWriteForbiddenError.Code.unknown_default_open_api).nullSafe())
            .add(
	            AiMedicationDoseSuggestion.Unit::class.java, EnumJsonAdapter.create(
		            AiMedicationDoseSuggestion.Unit::class.java)
                .withUnknownFallback(AiMedicationDoseSuggestion.Unit.unknown_default_open_api).nullSafe())
            .add(
	            AiMedicationDoseSuggestion.DefaultDoseCycle::class.java, EnumJsonAdapter.create(
		            AiMedicationDoseSuggestion.DefaultDoseCycle::class.java)
                .withUnknownFallback(AiMedicationDoseSuggestion.DefaultDoseCycle.unknown_default_open_api).nullSafe())
            .add(
	            AuthHouseholdMembership.Role::class.java, EnumJsonAdapter.create(
		            AuthHouseholdMembership.Role::class.java)
                .withUnknownFallback(AuthHouseholdMembership.Role.unknown_default_open_api).nullSafe())
            .add(
	            BackupZipResponseData.ContentType::class.java, EnumJsonAdapter.create(
		            BackupZipResponseData.ContentType::class.java)
                .withUnknownFallback(BackupZipResponseData.ContentType.unknown_default_open_api).nullSafe())
            .add(
	            Capabilities.Format::class.java, EnumJsonAdapter.create(Capabilities.Format::class.java)
                .withUnknownFallback(Capabilities.Format.unknown_default_open_api).nullSafe())
            .add(
	            Capabilities.ApiVersion::class.java, EnumJsonAdapter.create(Capabilities.ApiVersion::class.java)
                .withUnknownFallback(Capabilities.ApiVersion.unknown_default_open_api).nullSafe())
            .add(
	            Capabilities.PortableFormats::class.java, EnumJsonAdapter.create(Capabilities.PortableFormats::class.java)
                .withUnknownFallback(Capabilities.PortableFormats.unknown_default_open_api).nullSafe())
            .add(
	            CapabilityAuthentication.Methods::class.java, EnumJsonAdapter.create(
		            CapabilityAuthentication.Methods::class.java)
                .withUnknownFallback(CapabilityAuthentication.Methods.unknown_default_open_api).nullSafe())
            .add(
	            CapabilityAuthentication.HostedMobile::class.java, EnumJsonAdapter.create(
		            CapabilityAuthentication.HostedMobile::class.java)
                .withUnknownFallback(CapabilityAuthentication.HostedMobile.unknown_default_open_api).nullSafe())
            .add(
	            CapabilityAuthentication.PasswordLogin::class.java, EnumJsonAdapter.create(
		            CapabilityAuthentication.PasswordLogin::class.java)
                .withUnknownFallback(CapabilityAuthentication.PasswordLogin.unknown_default_open_api).nullSafe())
            .add(
	            CapabilityCli.Status::class.java, EnumJsonAdapter.create(CapabilityCli.Status::class.java)
                .withUnknownFallback(CapabilityCli.Status.unknown_default_open_api).nullSafe())
            .add(
	            CapabilityCli.Binary::class.java, EnumJsonAdapter.create(CapabilityCli.Binary::class.java)
                .withUnknownFallback(CapabilityCli.Binary.unknown_default_open_api).nullSafe())
            .add(
	            CapabilityCli.ApiBoundary::class.java, EnumJsonAdapter.create(CapabilityCli.ApiBoundary::class.java)
                .withUnknownFallback(CapabilityCli.ApiBoundary.unknown_default_open_api).nullSafe())
            .add(
	            CapabilityCli.Distribution::class.java, EnumJsonAdapter.create(CapabilityCli.Distribution::class.java)
                .withUnknownFallback(CapabilityCli.Distribution.unknown_default_open_api).nullSafe())
            .add(
	            CapabilityClientTools.Diagnostics::class.java, EnumJsonAdapter.create(
		            CapabilityClientTools.Diagnostics::class.java)
                .withUnknownFallback(CapabilityClientTools.Diagnostics.unknown_default_open_api).nullSafe())
            .add(
	            CapabilityFhir.Version::class.java, EnumJsonAdapter.create(CapabilityFhir.Version::class.java)
                .withUnknownFallback(CapabilityFhir.Version.unknown_default_open_api).nullSafe())
            .add(
	            CapabilityFhir.Resources::class.java, EnumJsonAdapter.create(CapabilityFhir.Resources::class.java)
                .withUnknownFallback(CapabilityFhir.Resources.unknown_default_open_api).nullSafe())
            .add(
	            CapabilityMcpServer.Transport::class.java, EnumJsonAdapter.create(
		            CapabilityMcpServer.Transport::class.java)
                .withUnknownFallback(CapabilityMcpServer.Transport.unknown_default_open_api).nullSafe())
            .add(
	            CapabilityMcpServer.Endpoint::class.java, EnumJsonAdapter.create(CapabilityMcpServer.Endpoint::class.java)
                .withUnknownFallback(CapabilityMcpServer.Endpoint.unknown_default_open_api).nullSafe())
            .add(
	            CapabilityMcpServer.StdioBinary::class.java, EnumJsonAdapter.create(
		            CapabilityMcpServer.StdioBinary::class.java)
                .withUnknownFallback(CapabilityMcpServer.StdioBinary.unknown_default_open_api).nullSafe())
            .add(
	            CapabilitySync.NumericIds::class.java, EnumJsonAdapter.create(CapabilitySync.NumericIds::class.java)
                .withUnknownFallback(CapabilitySync.NumericIds.unknown_default_open_api).nullSafe())
            .add(
	            DataExportResponseData.Format::class.java, EnumJsonAdapter.create(
		            DataExportResponseData.Format::class.java)
                .withUnknownFallback(DataExportResponseData.Format.unknown_default_open_api).nullSafe())
            .add(
	            DataExportResponseData.Scope::class.java, EnumJsonAdapter.create(
		            DataExportResponseData.Scope::class.java)
                .withUnknownFallback(DataExportResponseData.Scope.unknown_default_open_api).nullSafe())
            .add(
	            DataExportResponseData.Cipher::class.java, EnumJsonAdapter.create(
		            DataExportResponseData.Cipher::class.java)
                .withUnknownFallback(DataExportResponseData.Cipher.unknown_default_open_api).nullSafe())
            .add(
	            DataExportResponseData.Kdf::class.java, EnumJsonAdapter.create(
		            DataExportResponseData.Kdf::class.java)
                .withUnknownFallback(DataExportResponseData.Kdf.unknown_default_open_api).nullSafe())
            .add(
	            DataExportResponseData.ContentType::class.java, EnumJsonAdapter.create(
		            DataExportResponseData.ContentType::class.java)
                .withUnknownFallback(DataExportResponseData.ContentType.unknown_default_open_api).nullSafe())
            .add(
	            DosageOption.DefaultDoseCycle::class.java, EnumJsonAdapter.create(DosageOption.DefaultDoseCycle::class.java)
                .withUnknownFallback(DosageOption.DefaultDoseCycle.unknown_default_open_api).nullSafe())
            .add(
	            DosageOptionCreateAttributes.DefaultDoseCycle::class.java, EnumJsonAdapter.create(
		            DosageOptionCreateAttributes.DefaultDoseCycle::class.java)
                .withUnknownFallback(DosageOptionCreateAttributes.DefaultDoseCycle.unknown_default_open_api).nullSafe())
            .add(
	            DosageOptionUpdateAttributes.DefaultDoseCycle::class.java, EnumJsonAdapter.create(
		            DosageOptionUpdateAttributes.DefaultDoseCycle::class.java)
                .withUnknownFallback(DosageOptionUpdateAttributes.DefaultDoseCycle.unknown_default_open_api).nullSafe())
            .add(
	            HealthDataExportResponseData.Format::class.java, EnumJsonAdapter.create(
		            HealthDataExportResponseData.Format::class.java)
                .withUnknownFallback(HealthDataExportResponseData.Format.unknown_default_open_api).nullSafe())
            .add(
	            HealthDataExportResponseData.Scope::class.java, EnumJsonAdapter.create(
		            HealthDataExportResponseData.Scope::class.java)
                .withUnknownFallback(HealthDataExportResponseData.Scope.unknown_default_open_api).nullSafe())
            .add(
	            HealthEvent.EventKind::class.java, EnumJsonAdapter.create(HealthEvent.EventKind::class.java)
                .withUnknownFallback(HealthEvent.EventKind.unknown_default_open_api).nullSafe())
            .add(
	            HealthEvent.Severity::class.java, EnumJsonAdapter.create(HealthEvent.Severity::class.java)
                .withUnknownFallback(HealthEvent.Severity.unknown_default_open_api).nullSafe())
            .add(
	            HealthEventAttributes.EventKind::class.java, EnumJsonAdapter.create(
		            HealthEventAttributes.EventKind::class.java)
                .withUnknownFallback(HealthEventAttributes.EventKind.unknown_default_open_api).nullSafe())
            .add(
	            HealthEventAttributes.Severity::class.java, EnumJsonAdapter.create(
		            HealthEventAttributes.Severity::class.java)
                .withUnknownFallback(HealthEventAttributes.Severity.unknown_default_open_api).nullSafe())
            .add(
	            HealthEventCreateRequestHealthEvent.EventKind::class.java, EnumJsonAdapter.create(
		            HealthEventCreateRequestHealthEvent.EventKind::class.java)
                .withUnknownFallback(HealthEventCreateRequestHealthEvent.EventKind.unknown_default_open_api).nullSafe())
            .add(
	            HealthEventCreateRequestHealthEvent.Severity::class.java, EnumJsonAdapter.create(
		            HealthEventCreateRequestHealthEvent.Severity::class.java)
                .withUnknownFallback(HealthEventCreateRequestHealthEvent.Severity.unknown_default_open_api).nullSafe())
            .add(
	            HouseholdAdminSettings.SubscriptionPlan::class.java, EnumJsonAdapter.create(
		            HouseholdAdminSettings.SubscriptionPlan::class.java)
                .withUnknownFallback(HouseholdAdminSettings.SubscriptionPlan.unknown_default_open_api).nullSafe())
            .add(
	            HouseholdAdminSettingsAttributes.SubscriptionPlan::class.java, EnumJsonAdapter.create(
		            HouseholdAdminSettingsAttributes.SubscriptionPlan::class.java)
                .withUnknownFallback(HouseholdAdminSettingsAttributes.SubscriptionPlan.unknown_default_open_api).nullSafe())
            .add(
	            HouseholdInvitation.MembershipRole::class.java, EnumJsonAdapter.create(
		            HouseholdInvitation.MembershipRole::class.java)
                .withUnknownFallback(HouseholdInvitation.MembershipRole.unknown_default_open_api).nullSafe())
            .add(
	            HouseholdInvitationAttributes.MembershipRole::class.java, EnumJsonAdapter.create(
		            HouseholdInvitationAttributes.MembershipRole::class.java)
                .withUnknownFallback(HouseholdInvitationAttributes.MembershipRole.unknown_default_open_api).nullSafe())
            .add(
	            HouseholdMembership.Role::class.java, EnumJsonAdapter.create(HouseholdMembership.Role::class.java)
                .withUnknownFallback(HouseholdMembership.Role.unknown_default_open_api).nullSafe())
            .add(
	            HouseholdMembership.Status::class.java, EnumJsonAdapter.create(HouseholdMembership.Status::class.java)
                .withUnknownFallback(HouseholdMembership.Status.unknown_default_open_api).nullSafe())
            .add(
	            HouseholdMembershipAttributes.Role::class.java, EnumJsonAdapter.create(
		            HouseholdMembershipAttributes.Role::class.java)
                .withUnknownFallback(HouseholdMembershipAttributes.Role.unknown_default_open_api).nullSafe())
            .add(
	            HouseholdMembershipAttributes.Status::class.java, EnumJsonAdapter.create(
		            HouseholdMembershipAttributes.Status::class.java)
                .withUnknownFallback(HouseholdMembershipAttributes.Status.unknown_default_open_api).nullSafe())
            .add(
	            Me.MembershipRole::class.java, EnumJsonAdapter.create(Me.MembershipRole::class.java)
                .withUnknownFallback(Me.MembershipRole.unknown_default_open_api).nullSafe())
            .add(
	            MeAccount.Status::class.java, EnumJsonAdapter.create(MeAccount.Status::class.java)
                .withUnknownFallback(MeAccount.Status.unknown_default_open_api).nullSafe())
            .add(
	            Medication.ReorderStatus::class.java, EnumJsonAdapter.create(Medication.ReorderStatus::class.java)
                .withUnknownFallback(Medication.ReorderStatus.unknown_default_open_api).nullSafe())
            .add(
	            MedicationCreateAttributes.DoseUnit::class.java, EnumJsonAdapter.create(
		            MedicationCreateAttributes.DoseUnit::class.java)
                .withUnknownFallback(MedicationCreateAttributes.DoseUnit.unknown_default_open_api).nullSafe())
            .add(
	            MedicationCreateAttributes.DefaultScheduleType::class.java, EnumJsonAdapter.create(
		            MedicationCreateAttributes.DefaultScheduleType::class.java)
                .withUnknownFallback(MedicationCreateAttributes.DefaultScheduleType.unknown_default_open_api).nullSafe())
            .add(
	            MedicationLookupBarcodeResolution.Status::class.java, EnumJsonAdapter.create(
		            MedicationLookupBarcodeResolution.Status::class.java)
                .withUnknownFallback(MedicationLookupBarcodeResolution.Status.unknown_default_open_api).nullSafe())
            .add(
	            MedicationLookupReviewGuidance.Status::class.java, EnumJsonAdapter.create(
		            MedicationLookupReviewGuidance.Status::class.java)
                .withUnknownFallback(MedicationLookupReviewGuidance.Status.unknown_default_open_api).nullSafe())
            .add(
	            MedicationLookupReviewPrompt.RiskLevel::class.java, EnumJsonAdapter.create(
		            MedicationLookupReviewPrompt.RiskLevel::class.java)
                .withUnknownFallback(MedicationLookupReviewPrompt.RiskLevel.unknown_default_open_api).nullSafe())
            .add(
	            MedicationLookupReviewPrompt.MatchConfidence::class.java, EnumJsonAdapter.create(
		            MedicationLookupReviewPrompt.MatchConfidence::class.java)
                .withUnknownFallback(MedicationLookupReviewPrompt.MatchConfidence.unknown_default_open_api).nullSafe())
            .add(
	            MedicationLookupReviewPrompt.MatchType::class.java, EnumJsonAdapter.create(
		            MedicationLookupReviewPrompt.MatchType::class.java)
                .withUnknownFallback(MedicationLookupReviewPrompt.MatchType.unknown_default_open_api).nullSafe())
            .add(
	            MedicationLookupUnavailableError::class.java, EnumJsonAdapter.create(
		            MedicationLookupUnavailableError::class.java)
                .withUnknownFallback(MedicationLookupUnavailableError.unknown_default_open_api).nullSafe())
            .add(
	            MedicationTakeCreateRequestMedicationTake.SourceType::class.java, EnumJsonAdapter.create(
		            MedicationTakeCreateRequestMedicationTake.SourceType::class.java)
                .withUnknownFallback(MedicationTakeCreateRequestMedicationTake.SourceType.unknown_default_open_api).nullSafe())
            .add(
	            MedicationUpdateAttributes.DoseUnit::class.java, EnumJsonAdapter.create(
		            MedicationUpdateAttributes.DoseUnit::class.java)
                .withUnknownFallback(MedicationUpdateAttributes.DoseUnit.unknown_default_open_api).nullSafe())
            .add(
	            MedicationUpdateAttributes.DefaultScheduleType::class.java, EnumJsonAdapter.create(
		            MedicationUpdateAttributes.DefaultScheduleType::class.java)
                .withUnknownFallback(MedicationUpdateAttributes.DefaultScheduleType.unknown_default_open_api).nullSafe())
            .add(
	            NativeDeviceTokenAttributes.Platform::class.java, EnumJsonAdapter.create(
		            NativeDeviceTokenAttributes.Platform::class.java)
                .withUnknownFallback(NativeDeviceTokenAttributes.Platform.unknown_default_open_api).nullSafe())
            .add(
	            Person.PersonType::class.java, EnumJsonAdapter.create(Person.PersonType::class.java)
                .withUnknownFallback(Person.PersonType.unknown_default_open_api).nullSafe())
            .add(
	            PersonAccessGrant.AccessLevel::class.java, EnumJsonAdapter.create(PersonAccessGrant.AccessLevel::class.java)
                .withUnknownFallback(PersonAccessGrant.AccessLevel.unknown_default_open_api).nullSafe())
            .add(
	            PersonAccessGrant.RelationshipType::class.java, EnumJsonAdapter.create(
		            PersonAccessGrant.RelationshipType::class.java)
                .withUnknownFallback(PersonAccessGrant.RelationshipType.unknown_default_open_api).nullSafe())
            .add(
	            PersonAccessGrantAttributes.AccessLevel::class.java, EnumJsonAdapter.create(
		            PersonAccessGrantAttributes.AccessLevel::class.java)
                .withUnknownFallback(PersonAccessGrantAttributes.AccessLevel.unknown_default_open_api).nullSafe())
            .add(
	            PersonAccessGrantAttributes.RelationshipType::class.java, EnumJsonAdapter.create(
		            PersonAccessGrantAttributes.RelationshipType::class.java)
                .withUnknownFallback(PersonAccessGrantAttributes.RelationshipType.unknown_default_open_api).nullSafe())
            .add(
	            PersonAttributes.PersonType::class.java, EnumJsonAdapter.create(PersonAttributes.PersonType::class.java)
                .withUnknownFallback(PersonAttributes.PersonType.unknown_default_open_api).nullSafe())
            .add(
	            PersonCreateRequestPerson.PersonType::class.java, EnumJsonAdapter.create(
		            PersonCreateRequestPerson.PersonType::class.java)
                .withUnknownFallback(PersonCreateRequestPerson.PersonType.unknown_default_open_api).nullSafe())
            .add(
	            PersonMedication.DoseCycle::class.java, EnumJsonAdapter.create(PersonMedication.DoseCycle::class.java)
                .withUnknownFallback(PersonMedication.DoseCycle.unknown_default_open_api).nullSafe())
            .add(
	            PersonMedication.AdministrationKind::class.java, EnumJsonAdapter.create(
		            PersonMedication.AdministrationKind::class.java)
                .withUnknownFallback(PersonMedication.AdministrationKind.unknown_default_open_api).nullSafe())
            .add(
	            PersonMedicationAttributes.AdministrationKind::class.java, EnumJsonAdapter.create(
		            PersonMedicationAttributes.AdministrationKind::class.java)
                .withUnknownFallback(PersonMedicationAttributes.AdministrationKind.unknown_default_open_api).nullSafe())
            .add(
	            PersonMedicationAttributes.DoseCycle::class.java, EnumJsonAdapter.create(
		            PersonMedicationAttributes.DoseCycle::class.java)
                .withUnknownFallback(PersonMedicationAttributes.DoseCycle.unknown_default_open_api).nullSafe())
            .add(
	            PersonMedicationCreateRequestPersonMedication.AdministrationKind::class.java, EnumJsonAdapter.create(
		            PersonMedicationCreateRequestPersonMedication.AdministrationKind::class.java)
                .withUnknownFallback(PersonMedicationCreateRequestPersonMedication.AdministrationKind.unknown_default_open_api).nullSafe())
            .add(
	            PersonMedicationCreateRequestPersonMedication.DoseCycle::class.java, EnumJsonAdapter.create(
		            PersonMedicationCreateRequestPersonMedication.DoseCycle::class.java)
                .withUnknownFallback(PersonMedicationCreateRequestPersonMedication.DoseCycle.unknown_default_open_api).nullSafe())
            .add(
	            PersonMedicationReorderRequest.Direction::class.java, EnumJsonAdapter.create(
		            PersonMedicationReorderRequest.Direction::class.java)
                .withUnknownFallback(PersonMedicationReorderRequest.Direction.unknown_default_open_api).nullSafe())
            .add(
	            PortableEnvelope.Format::class.java, EnumJsonAdapter.create(PortableEnvelope.Format::class.java)
                .withUnknownFallback(PortableEnvelope.Format.unknown_default_open_api).nullSafe())
            .add(
	            PortableEnvelope.Cipher::class.java, EnumJsonAdapter.create(PortableEnvelope.Cipher::class.java)
                .withUnknownFallback(PortableEnvelope.Cipher.unknown_default_open_api).nullSafe())
            .add(
	            PortableEnvelope.Kdf::class.java, EnumJsonAdapter.create(PortableEnvelope.Kdf::class.java)
                .withUnknownFallback(PortableEnvelope.Kdf.unknown_default_open_api).nullSafe())
            .add(
	            PortableImportConflict.RecordType::class.java, EnumJsonAdapter.create(
		            PortableImportConflict.RecordType::class.java)
                .withUnknownFallback(PortableImportConflict.RecordType.unknown_default_open_api).nullSafe())
            .add(
	            PortableImportConflictField::class.java, EnumJsonAdapter.create(
		            PortableImportConflictField::class.java)
                .withUnknownFallback(PortableImportConflictField.unknown_default_open_api).nullSafe())
            .add(
	            PortableSnapshot.Format::class.java, EnumJsonAdapter.create(PortableSnapshot.Format::class.java)
                .withUnknownFallback(PortableSnapshot.Format.unknown_default_open_api).nullSafe())
            .add(
	            PortableSnapshot.Scope::class.java, EnumJsonAdapter.create(PortableSnapshot.Scope::class.java)
                .withUnknownFallback(PortableSnapshot.Scope.unknown_default_open_api).nullSafe())
            .add(
	            PushTestFailedError.Code::class.java, EnumJsonAdapter.create(PushTestFailedError.Code::class.java)
                .withUnknownFallback(PushTestFailedError.Code.unknown_default_open_api).nullSafe())
            .add(
	            PushTestFailedError.Message::class.java, EnumJsonAdapter.create(PushTestFailedError.Message::class.java)
                .withUnknownFallback(PushTestFailedError.Message.unknown_default_open_api).nullSafe())
            .add(
	            RateLimitError.Code::class.java, EnumJsonAdapter.create(RateLimitError.Code::class.java)
                .withUnknownFallback(RateLimitError.Code.unknown_default_open_api).nullSafe())
            .add(
	            Schedule.DoseCycle::class.java, EnumJsonAdapter.create(Schedule.DoseCycle::class.java)
                .withUnknownFallback(Schedule.DoseCycle.unknown_default_open_api).nullSafe())
            .add(
	            ScheduleAttributes.DoseCycle::class.java, EnumJsonAdapter.create(ScheduleAttributes.DoseCycle::class.java)
                .withUnknownFallback(ScheduleAttributes.DoseCycle.unknown_default_open_api).nullSafe())
            .add(
	            ScheduleAttributes.ScheduleType::class.java, EnumJsonAdapter.create(
		            ScheduleAttributes.ScheduleType::class.java)
                .withUnknownFallback(ScheduleAttributes.ScheduleType.unknown_default_open_api).nullSafe())
            .add(
	            ScheduleCreateRequestSchedule.DoseCycle::class.java, EnumJsonAdapter.create(
		            ScheduleCreateRequestSchedule.DoseCycle::class.java)
                .withUnknownFallback(ScheduleCreateRequestSchedule.DoseCycle.unknown_default_open_api).nullSafe())
            .add(
	            ScheduleCreateRequestSchedule.ScheduleType::class.java, EnumJsonAdapter.create(
		            ScheduleCreateRequestSchedule.ScheduleType::class.java)
                .withUnknownFallback(ScheduleCreateRequestSchedule.ScheduleType.unknown_default_open_api).nullSafe())
            .add(
	            SyncBatchOperation.Action::class.java, EnumJsonAdapter.create(SyncBatchOperation.Action::class.java)
                .withUnknownFallback(SyncBatchOperation.Action.unknown_default_open_api).nullSafe())
            .add(
	            SyncBatchOperation.ResourceType::class.java, EnumJsonAdapter.create(
		            SyncBatchOperation.ResourceType::class.java)
                .withUnknownFallback(SyncBatchOperation.ResourceType.unknown_default_open_api).nullSafe())
            .add(
	            SyncBatchResult.Action::class.java, EnumJsonAdapter.create(SyncBatchResult.Action::class.java)
                .withUnknownFallback(SyncBatchResult.Action.unknown_default_open_api).nullSafe())
            .add(
	            SyncBatchResult.RecordType::class.java, EnumJsonAdapter.create(SyncBatchResult.RecordType::class.java)
                .withUnknownFallback(SyncBatchResult.RecordType.unknown_default_open_api).nullSafe())
            .add(
	            SyncChange.Action::class.java, EnumJsonAdapter.create(SyncChange.Action::class.java)
                .withUnknownFallback(SyncChange.Action.unknown_default_open_api).nullSafe())
            .add(
	            SyncSnapshot.Format::class.java, EnumJsonAdapter.create(SyncSnapshot.Format::class.java)
                .withUnknownFallback(SyncSnapshot.Format.unknown_default_open_api).nullSafe())
            .add(
	            SyncSnapshot.Scope::class.java, EnumJsonAdapter.create(SyncSnapshot.Scope::class.java)
                .withUnknownFallback(SyncSnapshot.Scope.unknown_default_open_api).nullSafe())
            .add(
	            SyncTombstone.Action::class.java, EnumJsonAdapter.create(SyncTombstone.Action::class.java)
                .withUnknownFallback(SyncTombstone.Action.unknown_default_open_api).nullSafe())
>>>>>>> Stashed changes
    }
}
