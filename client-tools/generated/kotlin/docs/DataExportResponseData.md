
# DataExportResponseData

## Properties
| Name | Type | Description | Notes |
| ------------ | ------------- | ------------- | ------------- |
| **format** | [**inline**](#Format) |  |  [optional] |
| **scope** | [**inline**](#Scope) |  |  [optional] |
| **exportedAt** | [**java.time.OffsetDateTime**](java.time.OffsetDateTime.md) |  |  [optional] |
| **encryptedAt** | [**java.time.OffsetDateTime**](java.time.OffsetDateTime.md) |  |  [optional] |
| **sourceInstanceId** | **kotlin.String** |  |  [optional] |
| **records** | [**PortableRecords**](PortableRecords.md) |  |  [optional] |
| **cursor** | [**java.time.OffsetDateTime**](java.time.OffsetDateTime.md) |  |  [optional] |
| **cipher** | [**inline**](#Cipher) |  |  [optional] |
| **kdf** | [**inline**](#Kdf) |  |  [optional] |
| **salt** | **kotlin.String** |  |  [optional] |
| **checksum** | **kotlin.String** |  |  [optional] |
| **ciphertext** | **kotlin.String** |  |  [optional] |
| **filename** | **kotlin.String** |  |  [optional] |
| **contentType** | [**inline**](#ContentType) |  |  [optional] |
| **base64** | **kotlin.ByteArray** |  |  [optional] |


<a id="Format"></a>
## Enum: format
| Name | Value |
| ---- | ----- |
| format | medtracker.portable.v1, medtracker.portable.v2, medtracker.health_data.v1, medtracker.portable.encrypted.v1 |


<a id="Scope"></a>
## Enum: scope
| Name | Value |
| ---- | ----- |
| scope | single_person, household |


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


<a id="ContentType"></a>
## Enum: content_type
| Name | Value |
| ---- | ----- |
| contentType | application/zip |



