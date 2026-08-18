
# PortableEnvelope

## Properties
| Name | Type | Description | Notes |
| ------------ | ------------- | ------------- | ------------- |
| **format** | [**inline**](#Format) |  |  |
| **encryptedAt** | [**java.time.OffsetDateTime**](java.time.OffsetDateTime.md) |  |  |
| **cipher** | [**inline**](#Cipher) |  |  |
| **kdf** | [**inline**](#Kdf) |  |  |
| **salt** | **kotlin.String** |  |  |
| **checksum** | **kotlin.String** |  |  |
| **ciphertext** | **kotlin.String** |  |  |


<a id="Format"></a>
## Enum: format
| Name | Value |
| ---- | ----- |
| format | medtracker.portable.encrypted.v1 |


<a id="Cipher"></a>
## Enum: cipher
| Name | Value |
| ---- | ----- |
| cipher | aes-256-gcm |


<a id="Kdf"></a>
## Enum: kdf
| Name | Value |
| ---- | ----- |
| kdf | pbkdf2_sha256 |



