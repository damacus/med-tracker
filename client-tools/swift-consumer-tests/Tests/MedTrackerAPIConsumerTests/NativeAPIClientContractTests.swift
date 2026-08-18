import Foundation
import XCTest
@testable import MedTrackerAPI

final class NativeAPIClientContractTests: XCTestCase {
    func testFixedJSONValueTypesUseNativeSwiftTypes() throws {
        let medication = MedicationCreateAttributes(
            name: "Example medicine",
            reorderThreshold: "2.50",
            locationId: 3,
            doseAmount: "0.50",
            currentSupply: "10.00"
        )

        XCTAssertEqual(medication.reorderThreshold, "2.50")
        XCTAssertEqual(medication.doseAmount, "0.50")
        XCTAssertEqual(medication.currentSupply, "10.00")

        let identifiers = PersonMedicationAttributes(personId: "123", medicationId: "456")
        XCTAssertEqual(identifiers.personId, "123")
        XCTAssertEqual(identifiers.medicationId, "456")

        let take = MedicationTake(
            id: 1,
            portableId: UUID(uuidString: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa")!,
            clientUuid: UUID(uuidString: "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb"),
            scheduleId: nil,
            schedulePortableId: nil,
            personMedicationId: nil,
            personMedicationPortableId: nil,
            takenFromMedicationId: nil,
            takenFromMedicationPortableId: nil,
            takenFromLocationId: nil,
            takenFromLocationPortableId: nil,
            doseAmount: nil,
            doseUnit: nil,
            takenAt: Date(timeIntervalSince1970: 1_767_323_045),
            updatedAt: Date(timeIntervalSince1970: 1_767_323_045),
            personId: nil,
            personPortableId: nil,
            medicationId: nil,
            medicationPortableId: nil
        )

        XCTAssertNotNil(take.clientUuid)
        XCTAssertNotNil(take.takenAt)
    }

    func testNullableDecimalStringsDecodeAsOptionalStrings() throws {
        let decoder = JSONDecoder()

        let populated = try decoder.decode(
            MedicationCreateAttributes.self,
            from: Data(
                """
                {
                  "name": "Example medicine",
                  "reorder_threshold": "2.50",
                  "location_id": 3,
                  "dose_amount": "0.50",
                  "current_supply": "10.00"
                }
                """.utf8
            )
        )
        let populatedDoseAmount: String? = populated.doseAmount
        let populatedCurrentSupply: String? = populated.currentSupply
        XCTAssertEqual(populatedDoseAmount, "0.50")
        XCTAssertEqual(populatedCurrentSupply, "10.00")

        let explicitNull = try decoder.decode(
            MedicationCreateAttributes.self,
            from: Data(
                """
                {
                  "name": "Example medicine",
                  "reorder_threshold": "2.50",
                  "location_id": 3,
                  "dose_amount": null,
                  "current_supply": null
                }
                """.utf8
            )
        )
        let explicitNullDoseAmount: String? = explicitNull.doseAmount
        let explicitNullCurrentSupply: String? = explicitNull.currentSupply
        XCTAssertNil(explicitNullDoseAmount)
        XCTAssertNil(explicitNullCurrentSupply)

        let omitted = try decoder.decode(
            MedicationCreateAttributes.self,
            from: Data(
                """
                {
                  "name": "Example medicine",
                  "reorder_threshold": "2.50",
                  "location_id": 3
                }
                """.utf8
            )
        )
        let omittedDoseAmount: String? = omitted.doseAmount
        let omittedCurrentSupply: String? = omitted.currentSupply
        XCTAssertNil(omittedDoseAmount)
        XCTAssertNil(omittedCurrentSupply)
    }

    func testClearableRequestDecimalsRemainOptionalStrings() {
        let medicationCreate = MedicationCreateAttributes(
            name: "Example medicine",
            reorderThreshold: "2.50",
            locationId: 3
        )
        assertNilString(medicationCreate, \.doseAmount)
        assertNilString(medicationCreate, \.currentSupply)

        let medicationUpdate = MedicationUpdateAttributes()
        assertNilString(medicationUpdate, \.doseAmount)
        assertNilString(medicationUpdate, \.currentSupply)
        assertNilString(medicationUpdate, \.reorderThreshold)

        assertNilString(MedicationOrderDetailsRequestOrderDetails(), \.quantity)

        let dosageOptionCreate = DosageOptionCreateAttributes(
            medicationId: "123",
            amount: "0.50",
            unit: "tablet",
            frequency: "daily",
            defaultMaxDailyDoses: 1,
            defaultMinHoursBetweenDoses: "24",
            defaultDoseCycle: .daily
        )
        assertNilString(dosageOptionCreate, \.currentSupply)
        assertNilString(dosageOptionCreate, \.reorderThreshold)

        let dosageOptionUpdate = DosageOptionUpdateAttributes()
        assertNilString(dosageOptionUpdate, \.currentSupply)
        assertNilString(dosageOptionUpdate, \.reorderThreshold)

        let scheduleCreate = ScheduleCreateRequestSchedule(
            personId: "123",
            medicationId: "456",
            doseAmount: "0.50",
            doseUnit: "tablet",
            startDate: Date(timeIntervalSince1970: 1_767_323_045),
            endDate: Date(timeIntervalSince1970: 1_769_915_445)
        )
        assertNilString(scheduleCreate, \.minHoursBetweenDoses)
        assertNilString(ScheduleAttributes(), \.minHoursBetweenDoses)

        let personMedicationCreate = PersonMedicationCreateRequestPersonMedication(
            personId: "123",
            medicationId: "456"
        )
        assertNilString(personMedicationCreate, \.minHoursBetweenDoses)
        assertNilString(PersonMedicationAttributes(), \.minHoursBetweenDoses)
    }

    func testClearableRequestDecimalsEncodeOmissionExplicitNullAndValues() throws {
        let encoder = JSONEncoder()

        let omitted = try jsonObject(MedicationUpdateAttributes(), encoder: encoder)
        XCTAssertNil(omitted["current_supply"])

        let populated = try jsonObject(
            MedicationUpdateAttributes(currentSupply: "10.00"),
            encoder: encoder
        )
        XCTAssertEqual(populated["current_supply"] as? String, "10.00")

        var explicitNullModel = MedicationUpdateAttributes()
        explicitNullModel.clearCurrentSupply()
        let explicitNull = try jsonObject(explicitNullModel, encoder: encoder)
        XCTAssertTrue(explicitNull["current_supply"] is NSNull)

        var medicationUpdate = MedicationUpdateAttributes()
        medicationUpdate.clearDoseAmount()
        var orderDetails = MedicationOrderDetailsRequestOrderDetails()
        orderDetails.clearQuantity()
        var dosageOptionUpdate = DosageOptionUpdateAttributes()
        dosageOptionUpdate.clearCurrentSupply()
        dosageOptionUpdate.clearReorderThreshold()
        var scheduleUpdate = ScheduleAttributes()
        scheduleUpdate.clearMinHoursBetweenDoses()
        var personMedicationUpdate = PersonMedicationAttributes()
        personMedicationUpdate.clearMinHoursBetweenDoses()

        XCTAssertTrue(try jsonObject(medicationUpdate, encoder: encoder)["dose_amount"] is NSNull)
        XCTAssertTrue(try jsonObject(orderDetails, encoder: encoder)["quantity"] is NSNull)
        XCTAssertTrue(try jsonObject(dosageOptionUpdate, encoder: encoder)["current_supply"] is NSNull)
        XCTAssertTrue(try jsonObject(dosageOptionUpdate, encoder: encoder)["reorder_threshold"] is NSNull)
        XCTAssertTrue(
            try jsonObject(scheduleUpdate, encoder: encoder)["min_hours_between_doses"] is NSNull
        )
        XCTAssertTrue(
            try jsonObject(personMedicationUpdate, encoder: encoder)["min_hours_between_doses"] is NSNull
        )
    }

    func testPopulatedExplicitNullAndOmittedValuesDecode() throws {
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601

        let populated = try decoder.decode(
            MedicationTake.self,
            from: Data(
                """
                {
                  "id": 1,
                  "portable_id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
                  "client_uuid": "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
                  "taken_at": "2026-01-02T03:04:05Z",
                  "updated_at": "2026-01-02T03:04:05Z"
                }
                """.utf8
            )
        )
        XCTAssertEqual(populated.clientUuid?.uuidString.lowercased(), "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb")
        XCTAssertEqual(populated.takenAt, Date(timeIntervalSince1970: 1_767_323_045))

        let explicitNull = try decoder.decode(
            MedicationTake.self,
            from: Data(
                """
                {
                  "id": 1,
                  "portable_id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
                  "client_uuid": null,
                  "taken_at": null,
                  "updated_at": "2026-01-02T03:04:05Z"
                }
                """.utf8
            )
        )
        XCTAssertNil(explicitNull.clientUuid)
        XCTAssertNil(explicitNull.takenAt)

        let omitted = try decoder.decode(
            MedicationTake.self,
            from: Data(
                """
                {
                  "id": 1,
                  "portable_id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
                  "updated_at": "2026-01-02T03:04:05Z"
                }
                """.utf8
            )
        )
        XCTAssertNil(omitted.clientUuid)
        XCTAssertNil(omitted.takenAt)
    }

    func testMedicationLookupUsesOneResponseModelAndStringQuantities() throws {
        let decoder = JSONDecoder()

        let response = try decoder.decode(
            MedicationLookupResponse.self,
            from: Data(
                """
                {
                  "results": [],
                  "permissions": {"can_create": true, "can_update": false}
                }
                """.utf8
            )
        )
        XCTAssertTrue(response.results.isEmpty)
        XCTAssertTrue(response.permissions.canCreate)
        XCTAssertFalse(response.permissions.canUpdate)

        let result = try decoder.decode(
            MedicationLookupResult.self,
            from: Data(
                """
                {
                  "display": "Aspirin 300mg tablets",
                  "package_quantity": "1.5",
                  "related_medications": [],
                  "review_prompts": [],
                  "review_prompt_filter": {"hidden_count": 0}
                }
                """.utf8
            )
        )
        XCTAssertEqual(result.packageQuantity, "1.5")

        let nullResult = try decoder.decode(
            MedicationLookupResult.self,
            from: Data(
                """
                {
                  "display": "Aspirin 300mg tablets",
                  "package_quantity": null,
                  "related_medications": [],
                  "review_prompts": [],
                  "review_prompt_filter": {"hidden_count": 0}
                }
                """.utf8
            )
        )
        XCTAssertNil(nullResult.packageQuantity)
    }

    func testPortableImportConflictFieldHasStableWireValues() throws {
        let encoder = JSONEncoder()
        let decoder = JSONDecoder()

        XCTAssertEqual(String(data: try encoder.encode(PortableImportConflictField.field_name), encoding: .utf8), "\"name\"")
        XCTAssertEqual(String(data: try encoder.encode(PortableImportConflictField.email), encoding: .utf8), "\"email\"")
        XCTAssertEqual(try decoder.decode(PortableImportConflictField.self, from: Data("\"name\"".utf8)), .field_name)
        XCTAssertEqual(try decoder.decode(PortableImportConflictField.self, from: Data("\"email\"".utf8)), .email)
    }

    func testRepresentativeAuthenticatedAndErrorOperationsCompile() throws {
        let login = AuthenticationAPI.createLoginSessionWithRequestBuilder(
            authLoginRequest: AuthLoginRequest(email: "person@example.test", password: "password")
        )
        XCTAssertFalse(login.requiresAuthentication)
        XCTAssertEqual(login.method, "POST")
        XCTAssertTrue(login.URLString.hasSuffix("/auth/login"))

        let medication = MedicationCreateRequest(
            medication: MedicationCreateAttributes(
                name: "Example medicine",
                reorderThreshold: "2.50",
                locationId: 3,
                doseAmount: "0.50"
            )
        )
        let medicationRequest = MedicationsAPI.createMedicationWithRequestBuilder(
            householdId: 7,
            medicationCreateRequest: medication
        )
        XCTAssertTrue(medicationRequest.requiresAuthentication)
        XCTAssertEqual(
            MedicationsAPI.getMedicationWithRequestBuilder(householdId: 7, id: "123").URLString,
            "/api/v1/households/7/medications/123"
        )

        let sync = SyncBatchRequest(
            batch: SyncBatchRequestBatch(
                operations: [
                    SyncBatchOperation(action: .create, resourceType: .medication, id: "123"),
                ]
            )
        )
        let syncRequest = SynchronizationAPI.createSyncBatchWithRequestBuilder(householdId: 7, syncBatchRequest: sync)
        XCTAssertTrue(syncRequest.requiresAuthentication)
        XCTAssertTrue(
            SynchronizationAPI.getSyncChangesWithRequestBuilder(
                householdId: 7,
                cursor: Date(timeIntervalSince1970: 1_767_323_045)
            ).requiresAuthentication
        )

        let exportRequest = PortabilityAPI.getDataExportWithRequestBuilder(
            householdId: 7,
            mode: .healthDataJson,
            xMedTrackerPortablePassphrase: nil
        )
        XCTAssertTrue(exportRequest.requiresAuthentication)

        let error = try JSONDecoder().decode(
            ErrorEnvelope.self,
            from: Data(
                """
                {
                  "error": {
                    "code": "validation_error",
                    "message": "The request is invalid.",
                    "request_id": "request-123",
                    "errors": {"dose_amount": ["must be a string"]}
                  }
                }
                """.utf8
            )
        )
        XCTAssertEqual(error.error, ApiError(
            code: "validation_error",
            message: "The request is invalid.",
            requestId: "request-123",
            errors: ["dose_amount": ["must be a string"]]
        ))
    }

    func testGeneratedSourceHasNoKnownSwiftFailureShapes() throws {
        let sourceRoot = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .appendingPathComponent("generated/swift/Sources/MedTrackerAPI")
        let enumerator = FileManager.default.enumerator(
            at: sourceRoot,
            includingPropertiesForKeys: [.isRegularFileKey]
        )
        var swiftSources: [String] = []
        while let file = enumerator?.nextObject() as? URL {
            guard file.pathExtension == "swift" else { continue }
            swiftSources.append(try String(contentsOf: file, encoding: .utf8))
        }

        XCTAssertFalse(swiftSources.isEmpty)
        let source = swiftSources.joined(separator: "\n")
        XCTAssertNil(source.range(of: #"enum\s+\w+\s*:\s*Bool"#, options: .regularExpression))
        XCTAssertNil(source.range(of: #"enum\s+\w+\s*:\s*[^\n]*RawRepresentable.*Bool"#, options: .regularExpression))
        XCTAssertNil(source.range(of: #"https:\\\/\\/"#))
        XCTAssertNil(source.range(of: #"https:\/\/"#))
        XCTAssertNil(source.range(of: "ModelError"))
        XCTAssertTrue(source.contains("ApiError"))

        XCTAssertFalse(source.contains("public struct Nullable"))
        XCTAssertFalse(source.contains("public enum Nullable"))
    }

    private func assertNilString<Model>(
        _ model: Model,
        _ keyPath: KeyPath<Model, String?>,
        file: StaticString = #filePath,
        line: UInt = #line
    ) {
        XCTAssertNil(model[keyPath: keyPath], file: file, line: line)
    }

    private func jsonObject<Value: Encodable>(
        _ value: Value,
        encoder: JSONEncoder
    ) throws -> [String: Any] {
        let data = try encoder.encode(value)
        return try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [String: Any])
    }
}
