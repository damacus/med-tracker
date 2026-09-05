package io.damacus.medtracker.wear.protocol

import org.junit.Assert.*
import org.junit.Test

class CompanionProtocolTest {
    @Test fun `wire names and deterministic payload are stable`() {
        assertEquals("medtracker_phone_companion_v1", CompanionProtocol.CAPABILITY)
        assertEquals("/medtracker/companion/status", CompanionProtocol.STATUS_PATH)
        val status = CompanionStatus("1.0", SessionState.SIGNED_OUT, 123)
        val bytes = CompanionProtocol.encode(status)
        assertEquals("{\"protocolVersion\":1,\"phoneAppVersion\":\"1.0\",\"sessionState\":\"signed_out\",\"publishedAt\":123}", bytes.toString(Charsets.UTF_8))
        assertEquals(DecodedStatus.Supported(status), CompanionProtocol.decode(bytes))
        assertArrayEquals(bytes, CompanionProtocol.encode(status))
    }

    @Test fun `both session states and escaped versions round trip`() {
        SessionState.entries.forEach {
            val status = CompanionStatus("1.0\"\\test", it, 0)
            assertEquals(DecodedStatus.Supported(status), CompanionProtocol.decode(CompanionProtocol.encode(status)))
        }
    }

    @Test fun `unsupported versions are distinct from malformed data`() {
        assertEquals(DecodedStatus.Unsupported(2), CompanionProtocol.decode("{\"protocolVersion\":2}".toByteArray()))
        listOf("", "null", "[]", "{}", "{\"protocolVersion\":\"1\"}",
            "{\"protocolVersion\":1,\"phoneAppVersion\":\"1\",\"sessionState\":\"unknown\",\"publishedAt\":1}",
            "{\"protocolVersion\":1,\"phoneAppVersion\":\"1\",\"sessionState\":\"signed_in\",\"publishedAt\":-1}",
            "{\"protocolVersion\":1,\"phoneAppVersion\":1,\"sessionState\":\"signed_in\",\"publishedAt\":1}")
            .forEach { assertEquals(it, DecodedStatus.Malformed, CompanionProtocol.decode(it.toByteArray())) }
        assertEquals(DecodedStatus.Malformed, CompanionProtocol.decode(ByteArray(4097)))
    }
}
