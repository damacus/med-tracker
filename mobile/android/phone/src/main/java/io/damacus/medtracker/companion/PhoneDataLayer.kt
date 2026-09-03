package io.damacus.medtracker.companion

import android.content.Context
import com.google.android.gms.common.ConnectionResult
import com.google.android.gms.common.GoogleApiAvailability
import com.google.android.gms.common.api.ApiException
import com.google.android.gms.wearable.PutDataRequest
import com.google.android.gms.wearable.Wearable
import com.google.android.gms.wearable.WearableStatusCodes
import kotlinx.coroutines.tasks.await

class PhoneDataLayer(
    private val announce: suspend (String) -> Unit,
    private val put: suspend (String, ByteArray) -> Unit
) : CompanionTransport {
    constructor(context: Context) : this(
        announce = { capability ->
            check(GoogleApiAvailability.getInstance().isGooglePlayServicesAvailable(context) == ConnectionResult.SUCCESS)
            Wearable.getCapabilityClient(context).addLocalCapability(capability).await()
        },
        put = { path, data ->
            Wearable.getDataClient(context).putDataItem(
                PutDataRequest.create(path).setData(data).setUrgent()
            ).await()
        }
    )

    override suspend fun advertise(capability: String) {
        try {
            announce(capability)
        } catch (error: ApiException) {
            if (error.statusCode != WearableStatusCodes.DUPLICATE_CAPABILITY) throw error
        }
    }

    override suspend fun publish(path: String, data: ByteArray) {
        put(path, data)
    }
}
