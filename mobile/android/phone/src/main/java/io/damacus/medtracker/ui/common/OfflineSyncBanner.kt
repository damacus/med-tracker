package io.damacus.medtracker.ui.common

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CloudOff
import androidx.compose.material.icons.filled.Sync
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp

@Composable
fun OfflineSyncBanner(
    isOnline: Boolean,
    pendingMutationCount: Int
) {
    val showBanner = !isOnline || pendingMutationCount > 0

    AnimatedVisibility(visible = showBanner) {
        val backgroundColor = if (!isOnline) Color(0xFFD32F2F) else Color(0xFF1976D2)
        val icon = if (!isOnline) Icons.Default.CloudOff else Icons.Default.Sync
        val message = when {
            !isOnline && pendingMutationCount > 0 -> "Offline - $pendingMutationCount changes queued"
            !isOnline -> "Offline mode"
            else -> "Syncing $pendingMutationCount changes..."
        }

        Row(
            modifier = Modifier
                .fillMaxWidth()
                .background(backgroundColor)
                .padding(horizontal = 16.dp, vertical = 6.dp),
            horizontalArrangement = Arrangement.Center,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(
                imageVector = icon,
                contentDescription = null,
                tint = Color.White,
                modifier = Modifier.padding(end = 8.dp)
            )
            Text(
                text = message,
                style = MaterialTheme.typography.labelMedium,
                fontWeight = FontWeight.Bold,
                color = Color.White
            )
        }
    }
}
