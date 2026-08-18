package io.medtracker.client

import io.kotlintest.shouldBe
import io.kotlintest.specs.ShouldSpec
import io.medtracker.client.apis.AuthenticationApi
import io.medtracker.client.apis.MedicationsApi
import io.medtracker.client.apis.PortabilityApi
import io.medtracker.client.apis.SynchronizationApi
import io.medtracker.client.infrastructure.Serializer
import io.medtracker.client.models.ApiError
import io.medtracker.client.models.AuthLoginRequest
import io.medtracker.client.models.DosageOptionCreateAttributes
import io.medtracker.client.models.DosageOptionUpdateAttributes
import io.medtracker.client.models.ErrorEnvelope
import io.medtracker.client.models.Medication
import io.medtracker.client.models.MedicationCreateAttributes
import io.medtracker.client.models.MedicationCreateRequest
import io.medtracker.client.models.MedicationLookupResult
import io.medtracker.client.models.MedicationOrderDetailsRequestOrderDetails
import io.medtracker.client.models.MedicationTake
import io.medtracker.client.models.MedicationUpdateAttributes
import io.medtracker.client.models.PortableImportConflictField
import io.medtracker.client.models.PersonMedicationAttributes
import io.medtracker.client.models.PersonMedicationCreateRequestPersonMedication
import io.medtracker.client.models.ScheduleAttributes
import io.medtracker.client.models.ScheduleCreateRequestSchedule
import io.medtracker.client.models.SyncBatchOperation
import io.medtracker.client.models.SyncBatchRequest
import io.medtracker.client.models.SyncBatchRequestBatch
import java.time.OffsetDateTime
import java.util.UUID
import kotlin.reflect.KClass
import kotlin.reflect.full.memberProperties

private fun assertClearableDecimalEncoding(
    wireName: String,
    omitted: String,
    populated: String,
    explicitNull: String
) {
    omitted.contains("\"$wireName\"") shouldBe false
    populated shouldBe """{"$wireName":"10.00"}"""
    explicitNull shouldBe """{"$wireName":null}"""
}

class NativeApiClientContractTest : ShouldSpec() {
    init {
        should("expose fixed JSON value types as native Kotlin types") {
            propertyType(Medication::class, "reorderThreshold").classifier shouldBe String::class
            propertyType(Medication::class, "doseAmount").classifier shouldBe String::class
            propertyType(Medication::class, "doseAmount").isMarkedNullable shouldBe true
            propertyType(PersonMedicationAttributes::class, "personId").classifier shouldBe String::class
            propertyType(PersonMedicationAttributes::class, "personId").isMarkedNullable shouldBe true
            propertyType(MedicationTake::class, "clientUuid").classifier shouldBe UUID::class
            propertyType(MedicationTake::class, "clientUuid").isMarkedNullable shouldBe true
            propertyType(MedicationTake::class, "takenAt").classifier shouldBe OffsetDateTime::class
            propertyType(MedicationTake::class, "takenAt").isMarkedNullable shouldBe true
        }

        should("model clearable request decimals as nullable strings") {
            listOf(
                MedicationCreateAttributes::class to listOf("doseAmount", "currentSupply"),
                MedicationUpdateAttributes::class to listOf("doseAmount", "currentSupply", "reorderThreshold"),
                MedicationOrderDetailsRequestOrderDetails::class to listOf("quantity"),
                DosageOptionCreateAttributes::class to listOf("currentSupply", "reorderThreshold"),
                DosageOptionUpdateAttributes::class to listOf("currentSupply", "reorderThreshold"),
                ScheduleCreateRequestSchedule::class to listOf("minHoursBetweenDoses"),
                ScheduleAttributes::class to listOf("minHoursBetweenDoses"),
                PersonMedicationCreateRequestPersonMedication::class to listOf("minHoursBetweenDoses"),
                PersonMedicationAttributes::class to listOf("minHoursBetweenDoses")
            ).forEach { (model, propertyNames) ->
                propertyNames.forEach { propertyName ->
                    propertyType(model, propertyName).classifier shouldBe String::class
                    propertyType(model, propertyName).isMarkedNullable shouldBe true
                }
            }
        }

        should("not generate nullable wrapper model classes") {
            listOf(
                "NullableDecimalValue",
                "NullableNumericId",
                "NullablePortableId",
                "NullableTimestamp"
            ).forEach { modelName ->
                try {
                    Class.forName("io.medtracker.client.models.$modelName")
                    error("Unexpected generated wrapper model: $modelName")
                } catch (_: ClassNotFoundException) {
                }
            }
        }

        should("preserve the import conflict field wire values without a Kotlin name collision") {
            PortableImportConflictField.field_name.toString() shouldBe "name"
            PortableImportConflictField.email.toString() shouldBe "email"

            val adapter = Serializer.moshi.adapter(PortableImportConflictField::class.java)
            adapter.toJson(PortableImportConflictField.field_name) shouldBe "\"name\""
            adapter.fromJson("\"name\"") shouldBe PortableImportConflictField.field_name
        }

        should("decode populated, explicit-null, and omitted optional values") {
            val adapter = Serializer.moshi.adapter(MedicationCreateAttributes::class.java)

            val populated = adapter.fromJson(
                """
                {
                  "name": "Example medicine",
                  "reorder_threshold": "2.50",
                  "location_id": 3,
                  "dose_amount": "0.50",
                  "current_supply": "10.00"
                }
                """.trimIndent()
            )!!
            populated.reorderThreshold shouldBe "2.50"
            populated.doseAmount shouldBe "0.50"
            populated.currentSupply shouldBe "10.00"

            val explicitNull = adapter.fromJson(
                """
                {
                  "name": "Example medicine",
                  "reorder_threshold": "2.50",
                  "location_id": 3,
                  "dose_amount": null
                }
                """.trimIndent()
            )!!
            explicitNull.doseAmount shouldBe null

            val omitted = adapter.fromJson(
                """
                {
                  "name": "Example medicine",
                  "reorder_threshold": "2.50",
                  "location_id": 3
                }
                """.trimIndent()
            )!!
            omitted.doseAmount shouldBe null
        }

        should("encode clearable request decimals as omitted, null, or strings") {
            val medicationAdapter = Serializer.moshi.adapter(MedicationUpdateAttributes::class.java)
            assertClearableDecimalEncoding(
                wireName = "dose_amount",
                omitted = medicationAdapter.toJson(MedicationUpdateAttributes()),
                populated = medicationAdapter.toJson(MedicationUpdateAttributes(doseAmount = "10.00")),
                explicitNull = medicationAdapter.toJson(MedicationUpdateAttributes().clearDoseAmount())
            )
            assertClearableDecimalEncoding(
                wireName = "current_supply",
                omitted = medicationAdapter.toJson(MedicationUpdateAttributes()),
                populated = medicationAdapter.toJson(MedicationUpdateAttributes(currentSupply = "10.00")),
                explicitNull = medicationAdapter.toJson(MedicationUpdateAttributes().clearCurrentSupply())
            )

            val orderDetailsAdapter = Serializer.moshi.adapter(MedicationOrderDetailsRequestOrderDetails::class.java)
            assertClearableDecimalEncoding(
                wireName = "quantity",
                omitted = orderDetailsAdapter.toJson(MedicationOrderDetailsRequestOrderDetails()),
                populated = orderDetailsAdapter.toJson(
                    MedicationOrderDetailsRequestOrderDetails(quantity = "10.00")
                ),
                explicitNull = orderDetailsAdapter.toJson(
                    MedicationOrderDetailsRequestOrderDetails().clearQuantity()
                )
            )

            val dosageOptionAdapter = Serializer.moshi.adapter(DosageOptionUpdateAttributes::class.java)
            assertClearableDecimalEncoding(
                wireName = "current_supply",
                omitted = dosageOptionAdapter.toJson(DosageOptionUpdateAttributes()),
                populated = dosageOptionAdapter.toJson(DosageOptionUpdateAttributes(currentSupply = "10.00")),
                explicitNull = dosageOptionAdapter.toJson(DosageOptionUpdateAttributes().clearCurrentSupply())
            )
            assertClearableDecimalEncoding(
                wireName = "reorder_threshold",
                omitted = dosageOptionAdapter.toJson(DosageOptionUpdateAttributes()),
                populated = dosageOptionAdapter.toJson(DosageOptionUpdateAttributes(reorderThreshold = "10.00")),
                explicitNull = dosageOptionAdapter.toJson(
                    DosageOptionUpdateAttributes().clearReorderThreshold()
                )
            )

            val scheduleAdapter = Serializer.moshi.adapter(ScheduleAttributes::class.java)
            assertClearableDecimalEncoding(
                wireName = "min_hours_between_doses",
                omitted = scheduleAdapter.toJson(ScheduleAttributes()),
                populated = scheduleAdapter.toJson(ScheduleAttributes(minHoursBetweenDoses = "10.00")),
                explicitNull = scheduleAdapter.toJson(ScheduleAttributes().clearMinHoursBetweenDoses())
            )

            val personMedicationAdapter = Serializer.moshi.adapter(PersonMedicationAttributes::class.java)
            assertClearableDecimalEncoding(
                wireName = "min_hours_between_doses",
                omitted = personMedicationAdapter.toJson(PersonMedicationAttributes()),
                populated = personMedicationAdapter.toJson(
                    PersonMedicationAttributes(minHoursBetweenDoses = "10.00")
                ),
                explicitNull = personMedicationAdapter.toJson(
                    PersonMedicationAttributes().clearMinHoursBetweenDoses()
                )
            )
        }

        should("decode medication lookup package quantities as nullable strings") {
            val adapter = Serializer.moshi.adapter(MedicationLookupResult::class.java)

            val populated = adapter.fromJson(
                """
                {
                  "display": "Aspirin 300mg tablets",
                  "package_quantity": "1.5",
                  "related_medications": [],
                  "review_prompts": [],
                  "review_prompt_filter": {"hidden_count": 0}
                }
                """.trimIndent()
            )!!
            populated.packageQuantity shouldBe "1.5"

            val explicitNull = adapter.fromJson(
                """
                {
                  "display": "Aspirin 300mg tablets",
                  "package_quantity": null,
                  "related_medications": [],
                  "review_prompts": [],
                  "review_prompt_filter": {"hidden_count": 0}
                }
                """.trimIndent()
            )!!
            explicitNull.packageQuantity shouldBe null

            val omitted = adapter.fromJson(
                """
                {
                  "display": "Aspirin 300mg tablets",
                  "related_medications": [],
                  "review_prompts": [],
                  "review_prompt_filter": {"hidden_count": 0}
                }
                """.trimIndent()
            )!!
            omitted.packageQuantity shouldBe null
        }

        should("compile representative authenticated and error operations") {
            val authentication = AuthenticationApi()
            val loginConfig = authentication.createLoginSessionRequestConfig(
                AuthLoginRequest(email = "person@example.test", password = "password")
            )
            loginConfig.requiresAuthentication shouldBe false

            val medicationRequest = MedicationCreateRequest(
                medication = MedicationCreateAttributes(
                    name = "Example medicine",
                    reorderThreshold = "2.50",
                    locationId = 3,
                    doseAmount = "0.50"
                )
            )
            val medicationConfig = MedicationsApi().createMedicationRequestConfig(7, medicationRequest)
            medicationConfig.requiresAuthentication shouldBe true
            MedicationsApi().getMedicationRequestConfig(7, "123").path.endsWith("/medications/123") shouldBe true

            val syncRequest = SyncBatchRequest(
                batch = SyncBatchRequestBatch(
                    operations = listOf(
                        SyncBatchOperation(
                            action = SyncBatchOperation.Action.create,
                            resourceType = SyncBatchOperation.ResourceType.medication,
                            id = "123"
                        )
                    )
                )
            )
            SynchronizationApi().createSyncBatchRequestConfig(7, syncRequest).requiresAuthentication shouldBe true
            SynchronizationApi().getSyncChangesRequestConfig(
                7,
                OffsetDateTime.parse("2026-01-02T03:04:05Z")
            ).requiresAuthentication shouldBe true

            val exportConfig = PortabilityApi().getDataExportRequestConfig(
                householdId = 7,
                mode = PortabilityApi.ModeGetDataExport.health_data_json,
                xMedTrackerPortablePassphrase = null
            )
            exportConfig.requiresAuthentication shouldBe true

            val errorEnvelope = Serializer.moshi.adapter(ErrorEnvelope::class.java).fromJson(
                """
                {
                  "error": {
                    "code": "validation_error",
                    "message": "The request is invalid.",
                    "request_id": "request-123",
                    "errors": {"dose_amount": ["must be a string"]}
                  }
                }
                """.trimIndent()
            )!!
            errorEnvelope.error shouldBe ApiError(
                code = "validation_error",
                message = "The request is invalid.",
                requestId = "request-123",
                errors = mapOf("dose_amount" to listOf("must be a string"))
            )
        }
    }

    private fun propertyType(model: KClass<*>, propertyName: String) =
        model.memberProperties.single { it.name == propertyName }.returnType
}
