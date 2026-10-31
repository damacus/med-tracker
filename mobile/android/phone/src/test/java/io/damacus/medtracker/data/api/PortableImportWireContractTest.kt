package io.damacus.medtracker.data.api

import io.medtracker.client.infrastructure.Serializer
import io.medtracker.client.models.PortableImportUnprocessableResponse
import io.medtracker.client.models.PortableImportResult
import io.medtracker.client.models.ApiError
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Test

class PortableImportWireContractTest {
    private val adapter = Serializer.moshi.adapter(PortableImportUnprocessableResponse::class.java)

    @Test fun resultOnlyWireRetainsTypedFieldsWithoutAnError() {
        val response = requireNotNull(adapter.fromJson("""
            {"data":{"applied":false,"counts":{"people":2},"conflicts":[],"errors":["invalid"]}}
        """.trimIndent()))

        val result = requireNotNull(response.data)
        assertFalse(result.applied)
        assertEquals(2, result.counts["people"])
        assertEquals(listOf("invalid"), result.errors)
        assertNull(response.error)
    }

    @Test fun errorOnlyWireRetainsTypedFieldsWithoutAResult() {
        val response = requireNotNull(adapter.fromJson("""
            {"error":{"code":"invalid","message":"Invalid bundle","request_id":"request-1"}}
        """.trimIndent()))

        assertNull(response.data)
        assertEquals("invalid", response.error?.code)
        assertEquals("request-1", response.error?.requestId)
    }

    @Test fun generatedConstructorsKeepBothValidAlternativesUsable() {
        val result = PortableImportUnprocessableResponse(
            data = PortableImportResult(false, mapOf("people" to 2), emptyList(), listOf("invalid")),
            error = null
        )
        val error = PortableImportUnprocessableResponse(
            data = null,
            error = ApiError("invalid", "Invalid bundle", "request-1")
        )

        assertEquals(result, adapter.fromJson(adapter.toJson(result)))
        assertEquals(error, adapter.fromJson(adapter.toJson(error)))
    }
}
